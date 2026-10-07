//! Finds a usable Rust compiler (rustc).
//!
//! Like the JDK finder: an app opened from Finder doesn't get the shell's PATH,
//! so we look where rustup, Homebrew and manual installs put rustc.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEnv {
    pub rustc: PathBuf,
    pub version: String,
    /// Where we found it, e.g. "rustup" or "Homebrew".
    pub source: String,
}

/// Minimum version: the runtime uses let-else (1.65).
const MIN: (u32, u32) = (1, 65);

fn candidates() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        out.push((home.join(".cargo/bin/rustc"), "rustup".into()));
    }
    out.push((PathBuf::from("/opt/homebrew/opt/rustup/bin/rustc"), "Homebrew rustup".into()));
    out.push((PathBuf::from("/opt/homebrew/bin/rustc"), "Homebrew".into()));
    out.push((PathBuf::from("/usr/local/opt/rustup/bin/rustc"), "Homebrew rustup".into()));
    out.push((PathBuf::from("/usr/local/bin/rustc"), "/usr/local".into()));
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            out.push((dir.join("rustc"), "PATH".into()));
        }
    }
    out
}

/// "rustc 1.96.0 (ac68faa20 2026-05-25)" -> ("1.96.0", (1, 96))
pub fn parse_version(text: &str) -> Option<(String, (u32, u32))> {
    let v = text.split_whitespace().nth(1)?;
    let mut parts = v.split(['.', '-']);
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((v.to_string(), (major, minor)))
}

fn probe(rustc: &Path, source: &str) -> Result<RustEnv, String> {
    if !rustc.is_file() {
        return Err(format!("{}: not found", rustc.display()));
    }
    let out = Command::new(rustc).arg("--version").output().map_err(|e| format!("{}: {e}", rustc.display()))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("{}: {}", rustc.display(), err.lines().next().unwrap_or("failed")));
    }
    let (version, mm) = parse_version(&text).ok_or_else(|| format!("{}: unexpected version output {text:?}", rustc.display()))?;
    if mm < MIN {
        return Err(format!("{}: Rust {version} is too old, need {}.{}+", rustc.display(), MIN.0, MIN.1));
    }
    Ok(RustEnv { rustc: rustc.to_path_buf(), version, source: source.to_string() })
}

pub fn detect() -> Result<RustEnv, String> {
    let mut tried = Vec::new();
    for (path, source) in candidates() {
        match probe(&path, &source) {
            Ok(env) => return Ok(env),
            Err(e) => {
                if !tried.contains(&e) {
                    tried.push(e);
                }
            }
        }
    }
    Err(format!(
        "No Rust compiler found. Install Rust with `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` or `brew install rustup`.\nLooked in:\n{}",
        tried.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("rustc 1.96.0 (ac68faa20 2026-05-25)"), Some(("1.96.0".into(), (1, 96))));
        assert_eq!(parse_version("rustc 1.80.0-nightly (abc 2024-05-01)"), Some(("1.80.0-nightly".into(), (1, 80))));
        assert_eq!(parse_version("garbage"), None);
    }

    #[test]
    fn finds_rustc_on_this_machine() {
        let env = detect().expect("rustc should be installed for tests");
        assert!(env.rustc.is_file());
    }
}
