//! Finds a usable JDK.
//!
//! An app launched from Finder does not inherit the shell's PATH, and
//! `/usr/libexec/java_home` only knows JDKs installed under
//! /Library/Java. SDKMAN and Homebrew JDKs are invisible to it, so we scan the
//! usual install locations ourselves.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaEnv {
    pub home: PathBuf,
    pub java: PathBuf,
    pub javac: PathBuf,
    pub version: String,
    pub major: u32,
    /// Where we found it, e.g. "JAVA_HOME" or "SDKMAN".
    pub source: String,
}

impl JavaEnv {
    /// JDK 24 removed the SecurityManager (JEP 486). Older JDKs still allow it
    /// with `-Djava.security.manager=allow`.
    pub fn supports_security_manager(&self) -> bool {
        self.major < 24
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Every place a JDK commonly lives on macOS, in priority order.
pub fn candidate_homes(override_home: Option<&Path>) -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    if let Some(p) = override_home {
        out.push((p.to_path_buf(), "Settings".into()));
    }
    if let Some(h) = std::env::var_os("JAVA_HOME") {
        out.push((PathBuf::from(h), "JAVA_HOME".into()));
    }
    if let Some(home) = home_dir() {
        out.push((home.join(".sdkman/candidates/java/current"), "SDKMAN".into()));
        out.push((home.join(".asdf/installs/java"), "asdf".into()));
        push_children(&mut out, &home.join("Library/Java/JavaVirtualMachines"), "Contents/Home", "user JVMs");
    }
    for brew in ["/opt/homebrew/opt", "/usr/local/opt"] {
        out.push((PathBuf::from(brew).join("openjdk"), "Homebrew".into()));
        if let Ok(rd) = std::fs::read_dir(brew) {
            let mut names: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("openjdk@"))
                        .unwrap_or(false)
                })
                .collect();
            names.sort();
            names.reverse();
            for p in names {
                out.push((p, "Homebrew".into()));
            }
        }
    }
    push_children(&mut out, Path::new("/Library/Java/JavaVirtualMachines"), "Contents/Home", "system JVMs");
    if let Ok(o) = Command::new("/usr/libexec/java_home").output() {
        if o.status.success() {
            let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !p.is_empty() {
                out.push((PathBuf::from(p), "java_home".into()));
            }
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            if dir.starts_with("/usr/bin") {
                continue; // macOS stubs that only work when java_home works
            }
            if dir.join("javac").is_file() {
                if let Some(parent) = dir.parent() {
                    out.push((parent.to_path_buf(), "PATH".into()));
                }
            }
        }
    }
    out
}

fn push_children(out: &mut Vec<(PathBuf, String)>, dir: &Path, suffix: &str, label: &str) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path().join(suffix)).collect();
        v.sort();
        v.reverse();
        for p in v {
            out.push((p, label.to_string()));
        }
    }
}

/// Parses `openjdk version "17.0.8" 2023-07-18` or `java version "1.8.0_392"`.
pub fn parse_version(text: &str) -> Option<(String, u32)> {
    let start = text.find('"')? + 1;
    let end = start + text[start..].find('"')?;
    let v = text[start..end].to_string();
    let mut parts = v.split(|c: char| c == '.' || c == '_' || c == '-' || c == '+');
    let first: u32 = parts.next()?.parse().ok()?;
    let major = if first == 1 { parts.next()?.parse().ok()? } else { first };
    Some((v, major))
}

fn probe(home: &Path, source: &str) -> Option<JavaEnv> {
    let java = home.join("bin/java");
    let javac = home.join("bin/javac");
    if !java.is_file() || !javac.is_file() {
        return None;
    }
    let out = Command::new(&java).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    let (version, major) = parse_version(&text)?;
    // Resolve symlinks (SDKMAN's `current`) so logs show the real JDK.
    let home = std::fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    Some(JavaEnv {
        java: home.join("bin/java"),
        javac: home.join("bin/javac"),
        home,
        version,
        major,
        source: source.to_string(),
    })
}

/// Finds the first working JDK with javac and Java 11 or newer.
pub fn detect(override_home: Option<&Path>) -> Result<JavaEnv, String> {
    let mut tried = Vec::new();
    for (home, source) in candidate_homes(override_home) {
        match probe(&home, &source) {
            Some(env) if env.major >= 11 => return Ok(env),
            Some(env) => tried.push(format!("{} ({}): Java {} is too old, need 11+", home.display(), source, env.version)),
            None => tried.push(format!("{} ({}): no working java/javac", home.display(), source)),
        }
    }
    Err(format!(
        "No JDK found. Install one (for example `brew install openjdk@21`) or set its folder in Settings.\nLooked in:\n{}",
        tried.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("openjdk version \"17.0.8\" 2023-07-18"), Some(("17.0.8".into(), 17)));
        assert_eq!(parse_version("java version \"1.8.0_392\""), Some(("1.8.0_392".into(), 8)));
        assert_eq!(parse_version("openjdk version \"23\" 2024-09-17"), Some(("23".into(), 23)));
        assert_eq!(parse_version("nothing"), None);
    }

    #[test]
    fn finds_a_jdk_on_this_machine() {
        let env = detect(None).expect("a JDK should be installed for tests");
        assert!(env.javac.is_file());
        assert!(env.major >= 11);
    }
}
