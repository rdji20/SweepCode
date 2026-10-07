//! Client for the warm compiler process (PwChecker.java).
//!
//! One process serves both the live error check and the compile step of Run.
//! If it hangs or dies it is killed and started again on the next request.

use crate::java_env::JavaEnv;
use crate::support::{PREFIX, PREFIX_LINES};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: String,
    /// 1-based line in the user's code.
    pub line: i64,
    /// 1-based column.
    pub column: i64,
    /// UTF-16 offsets into the user's code, -1 when unknown.
    pub start: i64,
    pub end: i64,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResult {
    pub ok: bool,
    pub ms: u64,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Deserialize)]
struct RawResponse {
    ok: Option<bool>,
    ms: Option<u64>,
    diagnostics: Option<Vec<Diagnostic>>,
    fatal: Option<String>,
    extra: Option<String>,
}

struct Proc {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
    stderr_tail: Arc<Mutex<Vec<String>>>,
}

impl Drop for Proc {
    fn drop(&mut self) {
        unsafe {
            libc::killpg(self.child.id() as libc::pid_t, libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Checker {
    java: JavaEnv,
    support: PathBuf,
    scratch: PathBuf,
    proc: Option<Proc>,
    pub restarts: u32,
}

impl Checker {
    pub fn new(java: JavaEnv, support: PathBuf, scratch: PathBuf) -> Self {
        Checker { java, support, scratch, proc: None, restarts: 0 }
    }

    pub fn java(&self) -> &JavaEnv {
        &self.java
    }

    fn spawn(&mut self) -> Result<(), String> {
        std::fs::create_dir_all(&self.scratch).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&self.java.java);
        cmd.args(["-Xmx512m", "-XX:+UseSerialGC", "-XX:-UsePerfData", "-Xshare:auto", "-cp"])
            .arg(&self.support)
            .arg("PwChecker")
            .arg(&self.support)
            .arg(&self.scratch)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut child = cmd.spawn().map_err(|e| format!("cannot start the compiler process: {e}"))?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let tail = Arc::new(Mutex::new(Vec::new()));
        let tail2 = tail.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let mut t = tail2.lock().unwrap();
                t.push(text);
                if t.len() > 50 {
                    t.remove(0);
                }
            }
        });
        let p = Proc { child, stdin, lines: rx, stderr_tail: tail };
        let started = Instant::now();
        match p.lines.recv_timeout(Duration::from_secs(30)) {
            Ok(l) if l.contains("\"ready\"") => {
                log::info!("compiler process started (pid {}) in {} ms", p.child.id(), started.elapsed().as_millis());
                self.proc = Some(p);
                Ok(())
            }
            Ok(l) => Err(format!("compiler process failed to start: {l}")),
            Err(_) => {
                let err = p.stderr_tail.lock().unwrap().concat();
                Err(format!("compiler process did not start within 30s. stderr: {err}"))
            }
        }
    }

    fn alive(&mut self) -> bool {
        match self.proc.as_mut() {
            Some(p) => matches!(p.child.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// Starts the process early so the first check is fast.
    pub fn warm_up(&mut self) -> Result<(), String> {
        if !self.alive() {
            self.proc = None;
            self.spawn()?;
        }
        Ok(())
    }

    /// Compiles `user_code` (without our prefix) as `file_name`.
    /// With `out_dir` the class files are kept there for running.
    pub fn compile(&mut self, file_name: &str, user_code: &str, out_dir: Option<&Path>, timeout: Duration) -> Result<CompileResult, String> {
        let first = self.compile_once(file_name, user_code, out_dir, timeout);
        match first {
            Err(e) if e.starts_with("restart:") => {
                log::warn!("compiler process problem, restarting: {e}");
                self.restarts += 1;
                self.proc = None;
                self.compile_once(file_name, user_code, out_dir, timeout).map_err(|e| e.trim_start_matches("restart:").to_string())
            }
            other => other.map_err(|e| e.trim_start_matches("restart:").to_string()),
        }
    }

    fn compile_once(&mut self, file_name: &str, user_code: &str, out_dir: Option<&Path>, timeout: Duration) -> Result<CompileResult, String> {
        if !self.alive() {
            self.proc = None;
            self.spawn().map_err(|e| format!("restart:{e}"))?;
        }
        let source = format!("{PREFIX}{user_code}");
        let n = source.encode_utf16().count();
        let out = out_dir.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "-".into());
        if out.contains('\t') || out.contains('\n') || file_name.contains('\t') {
            return Err("paths with tabs or newlines are not supported".into());
        }
        let p = self.proc.as_mut().unwrap();
        // Drop any stale line left by a previous timed-out request.
        while p.lines.try_recv().is_ok() {}
        let req = format!("COMPILE\t{file_name}\t{out}\t{n}\n{source}");
        if let Err(e) = p.stdin.write_all(req.as_bytes()).and_then(|_| p.stdin.flush()) {
            return Err(format!("restart:cannot talk to the compiler process: {e}"));
        }
        let line = match p.lines.recv_timeout(timeout) {
            Ok(l) => l,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.proc = None; // killed by Drop
                return Err(format!("compiling took longer than {}s and was stopped", timeout.as_secs()));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let err = p.stderr_tail.lock().unwrap().concat();
                return Err(format!("restart:compiler process exited. stderr: {err}"));
            }
        };
        let raw: RawResponse = serde_json::from_str(&line).map_err(|e| format!("bad reply from compiler process: {e}: {line}"))?;
        if let Some(f) = raw.fatal {
            return Err(f);
        }
        if let Some(extra) = raw.extra.filter(|s| !s.trim().is_empty()) {
            log::debug!("javac extra output: {extra}");
        }
        let prefix_len = PREFIX.encode_utf16().count() as i64;
        let diagnostics = raw
            .diagnostics
            .unwrap_or_default()
            .into_iter()
            .map(|mut d| {
                d.line = (d.line - PREFIX_LINES).max(1);
                d.start = if d.start >= 0 { (d.start - prefix_len).max(0) } else { -1 };
                d.end = if d.end >= 0 { (d.end - prefix_len).max(0) } else { -1 };
                d
            })
            .collect();
        Ok(CompileResult { ok: raw.ok.unwrap_or(false), ms: raw.ms.unwrap_or(0), diagnostics })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{java_env, support};

    fn checker() -> (Checker, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let java = java_env::detect(None).unwrap();
        let classes = support::ensure_compiled(&java, dir.path()).unwrap();
        (Checker::new(java, classes, dir.path().join("scratch")), dir)
    }

    #[test]
    fn finds_lowercase_system_with_exact_range() {
        let (mut c, _d) = checker();
        let code = "class Solution {\n    void f() {\n        system.out.println(\"hi\");\n    }\n}\n";
        let r = c.compile("Solution.java", code, None, Duration::from_secs(30)).unwrap();
        assert!(!r.ok);
        assert_eq!(r.diagnostics.len(), 1);
        let d = &r.diagnostics[0];
        assert_eq!(d.line, 3);
        assert!(d.message.contains("system"), "{}", d.message);
        let s = &code[d.start as usize..d.end as usize];
        assert!(s.starts_with("system"), "range covers {s:?}");
    }

    #[test]
    fn clean_code_and_leetcode_imports() {
        let (mut c, _d) = checker();
        let code = "class Solution {\n  public List<Integer> f(TreeNode r, ListNode l) {\n    Map<Integer,Integer> m = new HashMap<>();\n    Pair<Integer,Integer> p = new Pair<>(1,2);\n    return new ArrayList<>();\n  }\n}\n";
        let r = c.compile("Solution.java", code, None, Duration::from_secs(30)).unwrap();
        assert!(r.ok, "{:?}", r.diagnostics);
    }

    #[test]
    fn missing_semicolon_and_restart_after_kill() {
        let (mut c, _d) = checker();
        let code = "class Solution {\n  int f() { return 1 }\n}\n";
        let r = c.compile("Solution.java", code, None, Duration::from_secs(30)).unwrap();
        assert_eq!(r.diagnostics[0].line, 2);
        assert!(r.diagnostics[0].message.contains("';' expected"));
        // Kill the daemon behind the client's back; next call must recover.
        let pid = c.proc.as_ref().unwrap().child.id();
        unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        std::thread::sleep(Duration::from_millis(100));
        let r = c.compile("Solution.java", "class Solution {}", None, Duration::from_secs(30)).unwrap();
        assert!(r.ok);
    }
}
