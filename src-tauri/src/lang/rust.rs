//! Rust: rustc for checks (check-only, ~0.4 s) and for runs (a generated main).
//!
//! The user's file is always `solution.rs`. A small wrapper `include!`s it after
//! the prelude, so compiler errors and panic locations point at the user's own
//! line numbers with no offset.

use super::{rust_codegen, Build, Language};
use crate::checker::{CompileResult, Diagnostic};
use crate::model::Meta;
use crate::proc;
use crate::runner::RunRequest;
use crate::rust_env::RustEnv;
use crate::store::Settings;
use regex::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

pub const PRELUDE: &str = include_str!("../../rust-runtime/prelude.rs");
pub const RUNTIME: &str = include_str!("../../rust-runtime/runtime.rs");
pub const FILE: &str = "solution.rs";

static LOC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"solution\.rs:(\d+)(?::\d+)?").unwrap());

pub struct Rust<'a> {
    pub env: RustEnv,
    /// Scratch folder for live checks.
    pub work: PathBuf,
    /// One check at a time (they share the scratch folder).
    pub lock: &'a Mutex<()>,
}

/// Converts a 1-based (line, char column) in `code` to a UTF-16 offset (what the editor uses).
fn utf16_offset(code: &str, line: i64, col: i64) -> i64 {
    let mut off = 0i64;
    for (i, l) in code.split('\n').enumerate() {
        if i as i64 + 1 == line {
            let chars: i64 = l.chars().take((col - 1).max(0) as usize).map(|c| c.len_utf16() as i64).sum();
            return off + chars;
        }
        off += l.encode_utf16().count() as i64 + 1;
    }
    off
}

/// Reads rustc's JSON diagnostics. Errors in the user's file become editor
/// diagnostics; anything else is returned separately.
fn parse_diagnostics(stderr: &str, code: &str) -> (Vec<Diagnostic>, Vec<String>) {
    let mut user = Vec::new();
    let mut other = Vec::new();
    for line in stderr.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let level = v["level"].as_str().unwrap_or("");
        if level != "error" && level != "warning" {
            continue;
        }
        let msg = v["message"].as_str().unwrap_or("").to_string();
        let spans = v["spans"].as_array().cloned().unwrap_or_default();
        let Some(primary) = spans.iter().find(|s| s["is_primary"].as_bool() == Some(true)) else {
            if level == "error" && !msg.starts_with("aborting due to") {
                other.push(msg);
            }
            continue;
        };
        let file = primary["file_name"].as_str().unwrap_or("");
        let in_user = file == FILE || file.ends_with("/solution.rs");
        let label = primary["label"].as_str().unwrap_or("");
        let full = if label.is_empty() || label == msg { msg.clone() } else { format!("{msg}: {label}") };
        if !in_user {
            if level == "error" {
                other.push(full);
            }
            continue;
        }
        let (ls, cs) = (primary["line_start"].as_i64().unwrap_or(1), primary["column_start"].as_i64().unwrap_or(1));
        let (le, ce) = (primary["line_end"].as_i64().unwrap_or(ls), primary["column_end"].as_i64().unwrap_or(cs));
        user.push(Diagnostic {
            severity: level.to_string(),
            line: ls,
            column: cs,
            start: utf16_offset(code, ls, cs),
            end: utf16_offset(code, le, ce),
            code: v["code"]["code"].as_str().unwrap_or("").to_string(),
            message: full,
        });
    }
    (user, other)
}

impl Rust<'_> {
    fn rustc(&self, dir: &Path, args: &[&str], timeout: Duration) -> Result<proc::ProcResult, String> {
        let mut cmd = Command::new(&self.env.rustc);
        cmd.args(args).current_dir(dir);
        let limits = proc::Limits { wall: timeout, max_stdout: 1 << 20, max_stderr: 4 << 20, ..proc::Limits::default() };
        proc::run(cmd, &limits, &AtomicBool::new(false)).map_err(|e| format!("cannot start rustc: {e}"))
    }

    fn check_in(&self, dir: &Path, code: &str) -> Result<CompileResult, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(FILE), code).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("check.rs"), format!("{PRELUDE}\ninclude!(\"{FILE}\");\n")).map_err(|e| e.to_string())?;
        let started = Instant::now();
        let r = self.rustc(
            dir,
            &["--edition", "2021", "--crate-type", "lib", "--emit=metadata", "--error-format=json", "-A", "warnings", "-o", "check.rmeta", "check.rs"],
            Duration::from_secs(30),
        )?;
        if r.termination == proc::Termination::TimedOut {
            return Err("rustc took longer than 30 s".into());
        }
        let (diagnostics, other) = parse_diagnostics(&r.stderr, code);
        if !other.is_empty() {
            log::warn!("rustc check: errors outside solution.rs: {}", other.join(" | "));
        }
        let ok = r.termination == (proc::Termination::Exited { code: 0 });
        Ok(CompileResult { ok, ms: started.elapsed().as_millis() as u64, diagnostics })
    }
}

impl Language for Rust<'_> {
    fn id(&self) -> &'static str {
        "rust"
    }
    fn name(&self) -> &'static str {
        "Rust"
    }
    fn toolchain(&self) -> String {
        format!("rustc {}", self.env.version)
    }
    fn file_name(&self, _meta: &Meta) -> String {
        FILE.to_string()
    }
    fn check(&self, _file_name: &str, code: &str) -> Result<CompileResult, String> {
        let _one = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        self.check_in(&self.work, code)
    }
    fn build(&self, req: &RunRequest, run_dir: &Path, _settings: &Settings) -> Build {
        let body = match rust_codegen::generate(&req.meta, &req.code) {
            Ok(b) => b,
            Err(why) => {
                // Usually a syntax error hides the method; show compiler errors if there are any.
                return match self.check_in(&run_dir.join("check"), &req.code) {
                    Ok(c) if !c.ok => Build::Failed(c),
                    _ => Build::Fatal(why),
                };
            }
        };
        let main = format!("{PRELUDE}\n{RUNTIME}\ninclude!(\"{FILE}\");\n\nfn main() {{\n{body}}}\n");
        if let Err(e) = std::fs::write(run_dir.join("main.rs"), main) {
            return Build::Fatal(format!("cannot write main.rs: {e}"));
        }
        let started = Instant::now();
        let r = match self.rustc(
            run_dir,
            &["--edition", "2021", "-C", "opt-level=1", "-C", "overflow-checks=on", "-C", "debuginfo=0", "--error-format=json", "-A", "warnings", "-o", "solution", "main.rs"],
            Duration::from_secs(90),
        ) {
            Ok(r) => r,
            Err(e) => return Build::Fatal(e),
        };
        let (diagnostics, other) = parse_diagnostics(&r.stderr, &req.code);
        let ok = r.termination == (proc::Termination::Exited { code: 0 });
        let compile = CompileResult { ok, ms: started.elapsed().as_millis() as u64, diagnostics };
        if !ok {
            if compile.diagnostics.iter().any(|d| d.severity == "error") {
                return Build::Failed(compile);
            }
            let first = other.first().cloned().unwrap_or_else(|| format!("rustc ended with {:?}", r.termination));
            log::warn!("rust build failed outside solution.rs: {}", other.join(" | "));
            return Build::Fatal(format!(
                "SweepCode can't call this method yet ({first}). If you changed the method's signature, keep the one LeetCode gave you."
            ));
        }
        Build::Ready {
            compile,
            program: run_dir.join("solution"),
            args: vec!["job.txt".into(), "results.jsonl".into(), _settings.memory_mb.to_string()],
            exec_dir: run_dir.to_path_buf(),
        }
    }
    fn map_trace(&self, trace: &[String], _file_name: &str) -> (Vec<String>, Option<i64>) {
        let first = trace.iter().find_map(|t| LOC.captures(t).and_then(|c| c[1].parse().ok()));
        (trace.to_vec(), first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust<'a>(lock: &'a Mutex<()>, dir: &Path) -> Rust<'a> {
        Rust { env: crate::rust_env::detect().unwrap(), work: dir.to_path_buf(), lock }
    }

    #[test]
    fn utf16_offsets() {
        let code = "ab\ncé𝄞d";
        assert_eq!(utf16_offset(code, 1, 1), 0);
        assert_eq!(utf16_offset(code, 2, 1), 3);
        assert_eq!(utf16_offset(code, 2, 4), 3 + 1 + 1 + 2);
    }

    #[test]
    fn live_check_points_at_the_user_line() {
        let lock = Mutex::new(());
        let dir = tempfile::tempdir().unwrap();
        let r = rust(&lock, dir.path());
        let code = "impl Solution {\n    pub fn f(n: i32) -> i32 {\n        let x: i32 = \"no\";\n        x + n\n    }\n}\n";
        let c = r.check(FILE, code).unwrap();
        assert!(!c.ok);
        let d = &c.diagnostics[0];
        assert_eq!(d.line, 3);
        assert_eq!(&code[d.start as usize..d.end as usize], "\"no\"");
        assert!(d.message.contains("mismatched types"), "{}", d.message);

        let good = "impl Solution {\n    pub fn f(n: i32) -> i32 {\n        let mut m: HashMap<i32, i32> = HashMap::new();\n        let h = BinaryHeap::from(vec![Reverse(1)]);\n        let _t: Option<Rc<RefCell<TreeNode>>> = None;\n        n + m.len() as i32 + h.len() as i32\n    }\n}\n";
        assert!(r.check(FILE, good).unwrap().ok);
        // A user who writes their own `use` lines doesn't clash with ours.
        let own = "use std::collections::HashMap;\nuse std::rc::Rc;\nimpl Solution { pub fn f() -> i32 { let _m: HashMap<i32, i32> = HashMap::new(); 0 } }\n";
        assert!(r.check(FILE, own).unwrap().ok, "{:?}", r.check(FILE, own).unwrap().diagnostics);
    }
}
