//! Runs a child process with hard limits so user code cannot hang or flood the app.
//!
//! - The child gets its own session/process group, so killing it also kills
//!   anything it spawned.
//! - Wall-clock timeout, cancel flag, and output caps all end in SIGKILL to the
//!   whole group.
//! - rlimits (CPU seconds, max file size, open files) are applied in the child
//!   before exec as a second line of defense.

use serde::Serialize;
use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Limits {
    pub wall: Duration,
    pub max_stdout: usize,
    pub max_stderr: usize,
    pub cpu_secs: Option<u64>,
    pub max_file_bytes: Option<u64>,
    pub max_open_files: Option<u64>,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            wall: Duration::from_secs(10),
            max_stdout: 1 << 20,
            max_stderr: 256 << 10,
            cpu_secs: None,
            max_file_bytes: Some(16 << 20),
            max_open_files: Some(256),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Termination {
    Exited { code: i32 },
    Signaled { signal: i32, name: String },
    TimedOut,
    OutputLimit,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcResult {
    pub termination: Termination,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub elapsed_ms: u64,
    pub pid: u32,
}

pub fn signal_name(sig: i32) -> String {
    match sig {
        libc::SIGKILL => "SIGKILL (killed)".into(),
        libc::SIGSEGV => "SIGSEGV (segmentation fault)".into(),
        libc::SIGABRT => "SIGABRT (aborted)".into(),
        libc::SIGXCPU => "SIGXCPU (CPU time limit)".into(),
        libc::SIGXFSZ => "SIGXFSZ (file size limit)".into(),
        libc::SIGBUS => "SIGBUS (bus error)".into(),
        libc::SIGTERM => "SIGTERM".into(),
        other => format!("signal {other}"),
    }
}

fn set_limit(resource: libc::c_int, soft: u64, hard: u64) {
    let lim = libc::rlimit { rlim_cur: soft as libc::rlim_t, rlim_max: hard as libc::rlim_t };
    // Safe to call between fork and exec. Failure just leaves the old limit.
    unsafe {
        libc::setrlimit(resource, &lim);
    }
}

fn kill_group(pid: u32) {
    unsafe {
        libc::killpg(pid as libc::pid_t, libc::SIGKILL);
    }
}

/// Reads a pipe to EOF, keeping at most `cap` bytes. Sets `over` once the cap is
/// passed but keeps draining so the child never blocks on a full pipe.
fn spawn_reader<R: Read + Send + 'static>(
    mut r: R,
    cap: usize,
    over: Arc<AtomicBool>,
) -> mpsc::Receiver<(Vec<u8>, bool)> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut truncated = false;
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let room = cap.saturating_sub(kept.len());
                    if n > room {
                        kept.extend_from_slice(&buf[..room]);
                        truncated = true;
                        over.store(true, Ordering::SeqCst);
                    } else {
                        kept.extend_from_slice(&buf[..n]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = tx.send((kept, truncated));
    });
    rx
}

/// Runs `cmd` to completion or until a limit hits. Never panics on child behavior.
pub fn run(mut cmd: Command, limits: &Limits, cancel: &AtomicBool) -> std::io::Result<ProcResult> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let cpu = limits.cpu_secs;
    let fsize = limits.max_file_bytes;
    let nofile = limits.max_open_files;
    unsafe {
        cmd.pre_exec(move || {
            // New session => new process group whose id is the child's pid.
            libc::setsid();
            if let Some(c) = cpu {
                set_limit(libc::RLIMIT_CPU, c, c + 1);
            }
            if let Some(f) = fsize {
                set_limit(libc::RLIMIT_FSIZE, f, f);
            }
            if let Some(n) = nofile {
                set_limit(libc::RLIMIT_NOFILE, n, n);
            }
            set_limit(libc::RLIMIT_CORE, 0, 0);
            Ok(())
        });
    }

    let start = Instant::now();
    let mut child = cmd.spawn()?;
    let pid = child.id();
    let over = Arc::new(AtomicBool::new(false));
    let out_rx = spawn_reader(child.stdout.take().unwrap(), limits.max_stdout, over.clone());
    let err_rx = spawn_reader(child.stderr.take().unwrap(), limits.max_stderr, over.clone());

    let mut forced: Option<Termination> = None;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if forced.is_none() {
            if cancel.load(Ordering::SeqCst) {
                forced = Some(Termination::Cancelled);
            } else if start.elapsed() >= limits.wall {
                forced = Some(Termination::TimedOut);
            } else if over.load(Ordering::SeqCst) {
                forced = Some(Termination::OutputLimit);
            }
            if forced.is_some() {
                kill_group(pid);
                let _ = child.kill();
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let elapsed_ms = start.elapsed().as_millis() as u64;
    // Clean up anything the child left running in its group.
    kill_group(pid);

    let wait = Duration::from_secs(2);
    let (stdout, stdout_truncated) = out_rx.recv_timeout(wait).unwrap_or_default();
    let (stderr, stderr_truncated) = err_rx.recv_timeout(wait).unwrap_or_default();

    let termination = match forced {
        Some(t) => t,
        None => {
            if over.load(Ordering::SeqCst) {
                Termination::OutputLimit
            } else if let Some(code) = status.code() {
                Termination::Exited { code }
            } else {
                let sig = status.signal().unwrap_or(0);
                Termination::Signaled { signal: sig, name: signal_name(sig) }
            }
        }
    };

    Ok(ProcResult {
        termination,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        stdout_truncated,
        stderr_truncated,
        elapsed_ms,
        pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> Command {
        let mut c = Command::new("/bin/sh");
        c.arg("-c").arg(script);
        c
    }

    fn quick() -> Limits {
        Limits { wall: Duration::from_millis(800), ..Limits::default() }
    }

    #[test]
    fn normal_exit_and_output() {
        let r = run(sh("echo hi; echo err >&2; exit 3"), &quick(), &AtomicBool::new(false)).unwrap();
        assert_eq!(r.termination, Termination::Exited { code: 3 });
        assert_eq!(r.stdout, "hi\n");
        assert_eq!(r.stderr, "err\n");
    }

    #[test]
    fn infinite_loop_times_out_and_children_die() {
        let marker = tempfile::tempdir().unwrap();
        let pidfile = marker.path().join("child.pid");
        // Parent spawns a background child that would outlive it, then spins.
        let script = format!("sleep 30 & echo $! > {}; while :; do :; done", pidfile.display());
        let r = run(sh(&script), &quick(), &AtomicBool::new(false)).unwrap();
        assert_eq!(r.termination, Termination::TimedOut);
        assert!(r.elapsed_ms < 3000);
        let child_pid: i32 = std::fs::read_to_string(&pidfile).unwrap().trim().parse().unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let alive = unsafe { libc::kill(child_pid, 0) } == 0;
        assert!(!alive, "background child should be killed with the group");
    }

    #[test]
    fn output_flood_is_capped() {
        let limits = Limits { max_stdout: 10_000, wall: Duration::from_secs(5), ..Limits::default() };
        let r = run(sh("yes spam"), &limits, &AtomicBool::new(false)).unwrap();
        assert_eq!(r.termination, Termination::OutputLimit);
        assert!(r.stdout.len() <= 10_000);
        assert!(r.stdout_truncated);
    }

    #[test]
    fn cancel_flag_kills() {
        let cancel = Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            c2.store(true, Ordering::SeqCst);
        });
        let limits = Limits { wall: Duration::from_secs(5), ..Limits::default() };
        let r = run(sh("sleep 10"), &limits, &cancel).unwrap();
        assert_eq!(r.termination, Termination::Cancelled);
        assert!(r.elapsed_ms < 2000);
    }

    #[test]
    fn cpu_rlimit_kills_spinner() {
        let limits = Limits { wall: Duration::from_secs(10), cpu_secs: Some(1), ..Limits::default() };
        let r = run(sh("while :; do :; done"), &limits, &AtomicBool::new(false)).unwrap();
        match r.termination {
            Termination::Signaled { signal, .. } => assert!(signal == libc::SIGXCPU || signal == libc::SIGKILL),
            other => panic!("expected signal, got {other:?}"),
        }
    }

    #[test]
    fn file_size_rlimit() {
        let dir = tempfile::tempdir().unwrap();
        let limits = Limits { max_file_bytes: Some(4096), wall: Duration::from_secs(5), ..Limits::default() };
        let script = format!("head -c 100000 /dev/zero > {}/big", dir.path().display());
        let r = run(sh(&script), &limits, &AtomicBool::new(false)).unwrap();
        let size = std::fs::metadata(dir.path().join("big")).map(|m| m.len()).unwrap_or(0);
        assert!(size <= 4096, "file grew to {size}");
        assert_ne!(r.termination, Termination::Exited { code: 0 });
    }
}
