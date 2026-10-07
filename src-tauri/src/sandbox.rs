//! macOS `sandbox-exec` wrapper: the OS-level jail for user programs.
//!
//! Even if the JVM-level guard is unavailable (JDK 24+), the program cannot
//! open network connections, fork or exec other programs, or write anywhere
//! except its own run folder.

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// True when sandbox-exec exists and accepts a profile on this macOS version.
pub fn available() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        Path::new(SANDBOX_EXEC).is_file()
            && Command::new(SANDBOX_EXEC)
                .args(["-p", "(version 1)(allow default)", "/usr/bin/true"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    })
}

fn quote(p: &Path) -> String {
    let s = p.to_string_lossy();
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Profile: allow everything by default, then deny network, fork, exec (except
/// the JDK's own binaries) and file writes (except the run folder).
pub fn profile(java_home: &Path, writable_dir: &Path) -> String {
    format!(
        r#"(version 1)
(allow default)
(deny network*)
(deny process-fork)
(deny process-exec)
(allow process-exec (subpath {jh}))
(deny file-write*)
(allow file-write* (subpath {wd}) (literal "/dev/null") (literal "/dev/zero") (literal "/dev/dtracehelper") (regex #"^/dev/tty"))
"#,
        jh = quote(java_home),
        wd = quote(writable_dir)
    )
}

/// Builds the command, wrapped in the sandbox when it is available.
/// Paths must be canonical (sandbox rules match real paths, e.g. /private/var).
pub fn command(program: &Path, java_home: &Path, writable_dir: &Path) -> (Command, bool) {
    if available() {
        let mut c = Command::new(SANDBOX_EXEC);
        c.arg("-p").arg(profile(java_home, writable_dir)).arg(program);
        (c, true)
    } else {
        (Command::new(program), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_writes_outside_and_allows_inside() {
        if !available() {
            eprintln!("sandbox-exec not available, skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let inside = std::fs::canonicalize(dir.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = std::fs::canonicalize(outside.path()).unwrap().join("x");
        let script = format!("echo ok > {}/a && echo bad > {}", inside.display(), outside_file.display());
        let (mut c, sandboxed) = command(Path::new("/bin/sh"), Path::new("/bin"), &inside);
        assert!(sandboxed);
        let out = c.arg("-c").arg(script).output().unwrap();
        assert!(inside.join("a").exists(), "write inside the run folder must work");
        assert!(!outside_file.exists(), "write outside must be blocked");
        assert!(!out.status.success());
    }

    #[test]
    fn blocks_exec_of_other_programs() {
        if !available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let inside = std::fs::canonicalize(dir.path()).unwrap();
        // Only /bin is allowed to exec; /usr/bin/env is not.
        let (mut c, _) = command(Path::new("/bin/sh"), Path::new("/bin"), &inside);
        let out = c.arg("-c").arg("/usr/bin/env true").output().unwrap();
        assert!(!out.status.success());
    }
}
