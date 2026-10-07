//! The Java helper classes (driver, checker, ListNode, TreeNode, Pair) are
//! embedded in the binary and compiled once per JDK into the cache folder.

use crate::java_env::JavaEnv;
use crate::proc;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Lines the user never sees, prepended to their code: LeetCode's implicit imports.
/// Kept on a single line so user line N is compiler line N + 1.
/// On-demand imports (`.*`) never clash with a class the user declares themselves.
pub const PREFIX: &str = "import java.util.*; import java.util.function.*; import java.util.stream.*; import java.math.*; import java.io.*; import javafx.util.*;\n";
pub const PREFIX_LINES: i64 = 1;

const SOURCES: &[(&str, &str)] = &[
    ("PwDriver.java", include_str!("../java/PwDriver.java")),
    ("PwChecker.java", include_str!("../java/PwChecker.java")),
    ("PwJson.java", include_str!("../java/PwJson.java")),
    ("ListNode.java", include_str!("../java/ListNode.java")),
    ("TreeNode.java", include_str!("../java/TreeNode.java")),
    ("javafx/util/Pair.java", include_str!("../java/javafx/util/Pair.java")),
];

/// Returns the folder with compiled support classes, compiling them if needed.
pub fn ensure_compiled(java: &JavaEnv, cache_dir: &Path) -> Result<PathBuf, String> {
    let mut h = Sha256::new();
    h.update(java.home.to_string_lossy().as_bytes());
    h.update(java.version.as_bytes());
    for (name, src) in SOURCES {
        h.update(name.as_bytes());
        h.update(src.as_bytes());
    }
    let hash: String = h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect();
    let root = cache_dir.join("support").join(&hash);
    let classes = root.join("classes");
    let stamp = root.join("ok");
    if stamp.is_file() {
        return Ok(classes);
    }
    let src_dir = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&classes).map_err(|e| format!("cannot create {}: {e}", classes.display()))?;
    let mut files = Vec::new();
    for (name, src) in SOURCES {
        let p = src_dir.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&p, src).map_err(|e| format!("cannot write {}: {e}", p.display()))?;
        files.push(p);
    }
    let mut cmd = Command::new(&java.javac);
    cmd.args(["-nowarn", "-Xlint:none", "-encoding", "UTF-8", "-d"]).arg(&classes).args(&files);
    let limits = proc::Limits { wall: Duration::from_secs(120), ..proc::Limits::default() };
    let r = proc::run(cmd, &limits, &AtomicBool::new(false)).map_err(|e| format!("cannot start javac: {e}"))?;
    if r.termination != (proc::Termination::Exited { code: 0 }) {
        return Err(format!("compiling helper classes failed ({:?}):\n{}{}", r.termination, r.stdout, r.stderr));
    }
    std::fs::write(&stamp, &java.version).map_err(|e| e.to_string())?;
    log::info!("compiled support classes into {}", classes.display());
    Ok(classes)
}

/// Removes old support builds (other JDKs / older app versions).
pub fn prune(cache_dir: &Path, keep: &Path) {
    let Some(keep_root) = keep.parent() else { return };
    if let Ok(rd) = std::fs::read_dir(cache_dir.join("support")) {
        for e in rd.flatten() {
            if e.path() != keep_root {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
}
