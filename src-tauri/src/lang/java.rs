//! Java: a warm javac process for checks, a reflection driver (PwDriver) for runs.

use super::{Build, Language};
use crate::checker::{Checker, CompileResult};
use crate::java_env::JavaEnv;
use crate::model::Meta;
use crate::runner::RunRequest;
use crate::store::Settings;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

static FRAME_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(([A-Za-z0-9_$]+\.java):(\d+)\)").unwrap());

pub struct Java<'a> {
    pub env: JavaEnv,
    pub support: PathBuf,
    pub checker: &'a Mutex<Option<Checker>>,
}

impl Java<'_> {
    fn compile(&self, file_name: &str, code: &str, out: Option<&Path>, timeout: Duration) -> Result<CompileResult, String> {
        let mut guard = self.checker.lock().unwrap_or_else(|p| p.into_inner());
        let checker = guard.as_mut().ok_or("the Java compiler is not running (no JDK found)")?;
        checker.compile(file_name, code, out, timeout)
    }

    fn jvm_args(&self, settings: &Settings) -> Vec<String> {
        let mut a = vec![
            format!("-Xmx{}m", settings.memory_mb),
            "-XX:+UseSerialGC".into(),
            "-XX:-UsePerfData".into(),
            "-Xshare:auto".into(),
            "-Dfile.encoding=UTF-8".into(),
            "-Djava.awt.headless=true".into(),
        ];
        if self.env.supports_security_manager() {
            a.push("-Djava.security.manager=allow".into());
        }
        a
    }
}

impl Language for Java<'_> {
    fn id(&self) -> &'static str {
        "java"
    }
    fn name(&self) -> &'static str {
        "Java"
    }
    fn toolchain(&self) -> String {
        format!("Java {}", self.env.version)
    }
    fn file_name(&self, meta: &Meta) -> String {
        meta.file_name()
    }
    fn check(&self, file_name: &str, code: &str) -> Result<CompileResult, String> {
        self.compile(file_name, code, None, Duration::from_secs(20))
    }
    fn build(&self, req: &RunRequest, run_dir: &Path, settings: &Settings) -> Build {
        let file_name = self.file_name(&req.meta);
        let compile = match self.compile(&file_name, &req.code, Some(run_dir), Duration::from_secs(30)) {
            Ok(c) => c,
            Err(e) => return Build::Fatal(format!("Compiler problem: {e}")),
        };
        if !compile.ok {
            return Build::Failed(compile);
        }
        let mut args = self.jvm_args(settings);
        args.push("-cp".into());
        args.push(format!("{}:{}", run_dir.display(), self.support.display()));
        args.push("PwDriver".into());
        args.push("job.txt".into());
        args.push("results.jsonl".into());
        Build::Ready { compile, program: self.env.java.clone(), args, exec_dir: self.env.home.clone() }
    }
    fn map_trace(&self, trace: &[String], file_name: &str) -> (Vec<String>, Option<i64>) {
        let mut first = None;
        let mapped = trace
            .iter()
            .map(|f| {
                FRAME_LINE
                    .replace_all(f, |c: &regex::Captures| {
                        let n: i64 = c[2].parse().unwrap_or(0);
                        if &c[1] == file_name {
                            let user = (n - crate::support::PREFIX_LINES).max(1);
                            if first.is_none() {
                                first = Some(user);
                            }
                            format!("({}:{})", &c[1], user)
                        } else {
                            c[0].to_string()
                        }
                    })
                    .to_string()
            })
            .collect();
        (mapped, first)
    }
}
