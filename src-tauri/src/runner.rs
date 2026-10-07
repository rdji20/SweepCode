//! Run pipeline: compile -> write job -> run sandboxed driver -> read results -> judge.
//!
//! Every run gets a folder `<cache>/runs/<run-id>/` holding the job, the raw
//! results, the class files and `report.json`. The newest 20 are kept so a
//! failing run can be inspected after the fact.

use crate::checker::CompileResult;
use crate::judge;
use crate::lang::{Build, Language};
use crate::model::{Meta, TestCase};
use crate::proc::{self, Termination};
use crate::sandbox;
use crate::store::Settings;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub const KEEP_RUNS: usize = 20;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub slug: String,
    pub code: String,
    pub meta: Meta,
    pub tests: Vec<TestCase>,
    #[serde(default)]
    pub any_order: bool,
    /// "java" or "rust".
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "java".into()
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Accepted,
    /// Ran fine but some tests have no expected output to compare with.
    Finished,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    OutputLimitExceeded,
    CompileError,
    InvalidInput,
    Cancelled,
    InternalError,
}

impl Verdict {
    pub fn key(&self) -> String {
        serde_json::to_value(self).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaseStatus {
    Passed,
    Failed,
    /// Ran without error, nothing to compare with.
    Ran,
    Error,
    Blocked,
    Timeout,
    Memory,
    OutputLimit,
    InputError,
    NotRun,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseResult {
    pub index: usize,
    pub labels: Vec<String>,
    pub inputs: Vec<String>,
    pub expected: Option<String>,
    pub output: Option<String>,
    pub stdout: String,
    pub stdout_truncated: bool,
    pub status: CaseStatus,
    pub error: Option<String>,
    pub error_kind: Option<String>,
    pub trace: Vec<String>,
    /// Line in the user's code where the error happened.
    pub error_line: Option<i64>,
    pub ms: Option<f64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub termination: Termination,
    pub elapsed_ms: u64,
    pub stderr: String,
    pub sandboxed: bool,
    /// The Java guard (SecurityManager) was active. Always false for Rust.
    pub guard: bool,
    /// e.g. "Java 17.0.8" or "rustc 1.96.0".
    pub toolchain: String,
    pub language: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    pub run_id: String,
    pub slug: String,
    pub language: String,
    pub started_at: String,
    pub verdict: Verdict,
    pub summary: String,
    pub message: Option<String>,
    pub compile: Option<CompileResult>,
    pub cases: Vec<CaseResult>,
    pub process: Option<ProcessInfo>,
    pub total_ms: u64,
    pub run_dir: String,
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn new_run_id() -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst) % 1000;
    format!("{}-{:03}", chrono::Local::now().format("%Y%m%d-%H%M%S"), n)
}

/// JVM noise from installing the SecurityManager on JDK 17-23.
fn clean_stderr(s: &str) -> String {
    s.lines()
        .filter(|l| {
            !(l.starts_with("WARNING: A terminally deprecated method")
                || l.starts_with("WARNING: System::setSecurityManager")
                || l.starts_with("WARNING: Please consider reporting this to the maintainers"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn job_file(meta: &Meta, tests: &[TestCase]) -> String {
    let mut s = String::new();
    match meta {
        Meta::Function { method, params, output_param, .. } => {
            s.push_str("mode=function\n");
            s.push_str(&format!("method={method}\nparams={}\n", params.len()));
            s.push_str(&format!("outputParam={}\n", output_param.map(|x| x as i64).unwrap_or(-1)));
            s.push_str(&format!("paramNames={}\n", params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(",")));
        }
        Meta::Design { class_name, .. } => {
            s.push_str(&format!("mode=design\nclass={class_name}\n"));
        }
    }
    s.push_str("---\n");
    for t in tests {
        for line in &t.inputs {
            // One value per line: newlines inside a value would shift every later input.
            s.push_str(&line.replace(['\r', '\n'], " "));
            s.push('\n');
        }
    }
    s
}

fn shell_words(args: &[String]) -> String {
    args.iter()
        .map(|a| if a.contains(' ') || a.contains('(') || a.contains('"') { format!("'{}'", a.replace('\'', "'\\''")) } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn prune_runs(runs_dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(runs_dir) else { return };
    let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    if dirs.len() > keep {
        for d in &dirs[..dirs.len() - keep] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

pub struct RunEnv<'a> {
    pub lang: &'a dyn Language,
    pub runs_dir: &'a Path,
    pub settings: &'a Settings,
}

/// What a crashed process left in the per-test capture file (Rust runs only).
fn crash_output(run_dir: &Path) -> String {
    std::fs::read(run_dir.join("case-output.txt"))
        .map(|b| String::from_utf8_lossy(&b[..b.len().min(64 * 1024)]).to_string())
        .unwrap_or_default()
}

pub fn run(env: &RunEnv, req: &RunRequest, cancel: &AtomicBool) -> RunReport {
    let started = Instant::now();
    let run_id = new_run_id();
    let started_at = chrono::Local::now().to_rfc3339();
    let run_dir = env.runs_dir.join(&run_id);
    let lang = env.lang;
    let file_name = lang.file_name(&req.meta);
    let labels = req.meta.input_labels();
    let per = req.meta.lines_per_test();
    log::info!("run {run_id}: start slug={} tests={} file={file_name} toolchain={} timeout={}s mem={}MB", req.slug, req.tests.len(), lang.toolchain(), env.settings.timeout_secs, env.settings.memory_mb);

    let mut report = RunReport {
        run_id: run_id.clone(),
        slug: req.slug.clone(),
        language: lang.id().to_string(),
        started_at,
        verdict: Verdict::InternalError,
        summary: String::new(),
        message: None,
        compile: None,
        cases: req
            .tests
            .iter()
            .enumerate()
            .map(|(i, t)| CaseResult {
                index: i,
                labels: labels.clone(),
                inputs: t.inputs.clone(),
                expected: t.expected.clone().filter(|e| !e.trim().is_empty()),
                output: None,
                stdout: String::new(),
                stdout_truncated: false,
                status: CaseStatus::NotRun,
                error: None,
                error_kind: None,
                trace: vec![],
                error_line: None,
                ms: None,
                note: None,
            })
            .collect(),
        process: None,
        total_ms: 0,
        run_dir: run_dir.display().to_string(),
    };

    let finish = |mut r: RunReport, started: Instant| -> RunReport {
        r.total_ms = started.elapsed().as_millis() as u64;
        if r.summary.is_empty() {
            let passed = r.cases.iter().filter(|c| c.status == CaseStatus::Passed).count();
            let with_expected = r.cases.iter().filter(|c| c.expected.is_some()).count();
            r.summary = if with_expected > 0 { format!("{passed}/{with_expected} passed") } else { format!("{} ran", r.cases.len()) };
        }
        let _ = std::fs::write(Path::new(&r.run_dir).join("report.json"), serde_json::to_string_pretty(&r).unwrap_or_default());
        log::info!("run {}: {} ({}) in {} ms", r.run_id, r.verdict.key(), r.summary, r.total_ms);
        r
    };

    if let Err(e) = std::fs::create_dir_all(&run_dir) {
        report.message = Some(format!("cannot create run folder {}: {e}", run_dir.display()));
        log::error!("run {run_id}: {}", report.message.as_ref().unwrap());
        return finish(report, started);
    }
    let run_dir = std::fs::canonicalize(&run_dir).unwrap_or(run_dir);
    let _ = std::fs::write(run_dir.join(&file_name), &req.code);

    // Test cases must have one line per parameter.
    let bad: Vec<usize> = req.tests.iter().enumerate().filter(|(_, t)| t.inputs.len() != per).map(|(i, _)| i).collect();
    if req.tests.is_empty() || !bad.is_empty() {
        report.verdict = Verdict::InvalidInput;
        report.message = Some(if req.tests.is_empty() {
            "There are no test cases. Add one in the Tests tab.".into()
        } else {
            format!("Test {} needs exactly {per} input line(s).", bad[0] + 1)
        });
        for i in bad {
            report.cases[i].status = CaseStatus::InputError;
        }
        return finish(report, started);
    }

    // ---- build (language specific)
    if let Err(e) = std::fs::write(run_dir.join("job.txt"), job_file(&req.meta, &req.tests)) {
        report.message = Some(format!("cannot write job file: {e}"));
        return finish(report, started);
    }
    let (compile, program, args, exec_dir) = match lang.build(req, &run_dir, env.settings) {
        Build::Ready { compile, program, args, exec_dir } => (compile, program, args, exec_dir),
        Build::Failed(compile) => {
            log::info!("run {run_id}: compiled ok=false in {} ms, {} diagnostics", compile.ms, compile.diagnostics.len());
            for d in compile.diagnostics.iter().filter(|d| d.severity == "error").take(5) {
                log::info!("run {run_id}: compile error line {}: {}", d.line, d.message.lines().next().unwrap_or(""));
            }
            let n = compile.diagnostics.iter().filter(|d| d.severity == "error").count();
            report.summary = format!("{n} error{}", if n == 1 { "" } else { "s" });
            report.compile = Some(compile);
            report.verdict = Verdict::CompileError;
            return finish(report, started);
        }
        Build::Fatal(msg) => {
            log::error!("run {run_id}: build problem: {msg}");
            report.message = Some(msg);
            return finish(report, started);
        }
    };
    log::info!("run {run_id}: compiled ok in {} ms", compile.ms);
    report.compile = Some(compile);
    if cancel.load(Ordering::SeqCst) {
        report.verdict = Verdict::Cancelled;
        return finish(report, started);
    }

    // ---- execute
    let exec_dir = std::fs::canonicalize(&exec_dir).unwrap_or(exec_dir);
    let (mut cmd, sandboxed) = sandbox::command(&program, &exec_dir, &run_dir);
    cmd.args(&args)
        .current_dir(&run_dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "en_US.UTF-8")
        .env("HOME", &run_dir)
        .env("TMPDIR", &run_dir);
    let command_line = format!("{}{} {}", if sandboxed { "sandbox-exec -p <profile> " } else { "" }, program.display(), shell_words(&args));
    log::debug!("run {run_id}: {command_line}");
    if !sandboxed {
        log::warn!("run {run_id}: sandbox-exec unavailable, running with process limits only");
    }
    let timeout = Duration::from_secs(env.settings.timeout_secs);
    let limits = proc::Limits {
        wall: timeout,
        max_stdout: 256 << 10,
        max_stderr: 256 << 10,
        cpu_secs: Some(env.settings.timeout_secs * 4 + 5),
        max_file_bytes: Some(64 << 20),
        max_open_files: Some(512),
    };
    let pr = match proc::run(cmd, &limits, cancel) {
        Ok(p) => p,
        Err(e) => {
            report.message = Some(format!("could not start {}: {e}", lang.name()));
            log::error!("run {run_id}: spawn failed: {e}");
            return finish(report, started);
        }
    };
    let stderr = clean_stderr(&pr.stderr);
    log::info!("run {run_id}: process ended {:?} after {} ms (sandboxed={sandboxed})", pr.termination, pr.elapsed_ms);
    if !stderr.trim().is_empty() {
        log::warn!("run {run_id}: stderr: {}", stderr.chars().take(2000).collect::<String>());
    }

    // ---- read results
    let raw = std::fs::read_to_string(run_dir.join("results.jsonl")).unwrap_or_default();
    let mut guard_on = false;
    let mut fatal: Option<String> = None;
    let mut started_idx: Option<usize> = None;
    let mut ended = false;
    for line in raw.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            log::warn!("run {run_id}: unreadable result line: {}", line.chars().take(200).collect::<String>());
            continue;
        };
        match v["t"].as_str() {
            Some("meta") => guard_on = v["guard"].as_bool().unwrap_or(false),
            Some("start") => started_idx = v["i"].as_u64().map(|x| x as usize),
            Some("fatal") => fatal = v["error"].as_str().map(String::from),
            Some("end") => ended = true,
            Some("done") => {
                let Some(i) = v["i"].as_u64().map(|x| x as usize) else { continue };
                let Some(c) = report.cases.get_mut(i) else { continue };
                if started_idx == Some(i) {
                    started_idx = None;
                }
                c.stdout = v["stdout"].as_str().unwrap_or("").to_string();
                c.stdout_truncated = v["stdoutTruncated"].as_bool().unwrap_or(false);
                c.ms = v["ms"].as_f64();
                if v["status"] == "ok" {
                    let out = v["output"].as_str().unwrap_or("").to_string();
                    c.status = match &c.expected {
                        Some(exp) => {
                            let m = judge::compare(&out, exp, req.any_order);
                            c.note = m.note();
                            if m.passed() { CaseStatus::Passed } else { CaseStatus::Failed }
                        }
                        None => CaseStatus::Ran,
                    };
                    c.output = Some(out);
                } else {
                    let kind = v["errorKind"].as_str().unwrap_or("exception").to_string();
                    let trace: Vec<String> = v["trace"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                    let (trace, line) = lang.map_trace(&trace, &file_name);
                    c.status = match kind.as_str() {
                        "memory" => CaseStatus::Memory,
                        "blocked" => CaseStatus::Blocked,
                        "input" => CaseStatus::InputError,
                        _ => CaseStatus::Error,
                    };
                    c.error = v["error"].as_str().map(String::from);
                    c.error_kind = Some(kind);
                    c.trace = trace;
                    c.error_line = line;
                }
            }
            _ => {}
        }
    }

    // A test that started but never finished was the one running when the process ended.
    let in_progress = started_idx;
    let term = pr.termination.clone();
    match &term {
        Termination::TimedOut => {
            if let Some(i) = in_progress {
                report.cases[i].status = CaseStatus::Timeout;
                report.cases[i].error = Some(format!("Still running after {}s (infinite loop or too slow). The program was stopped.", env.settings.timeout_secs));
            } else if !ended {
                report.message = Some(format!("{} did not finish within {}s.", lang.name(), env.settings.timeout_secs));
            }
        }
        Termination::Signaled { signal, name } => {
            let mut status = if *signal == libc::SIGXCPU { CaseStatus::Timeout } else if *signal == libc::SIGXFSZ { CaseStatus::OutputLimit } else { CaseStatus::Error };
            if let Some(i) = in_progress {
                // A native program (Rust) writes why it aborted into the capture file.
                let printed = crash_output(&run_dir);
                let mut why = format!("The program was killed by {name}.");
                if printed.contains("memory allocation of") {
                    status = CaseStatus::Memory;
                    why = format!("Used more than {} MB of memory.", env.settings.memory_mb);
                } else if printed.contains("has overflowed its stack") {
                    why = "Stack overflow: the recursion went too deep.".into();
                }
                let c = &mut report.cases[i];
                c.status = status;
                c.error = Some(why);
                if c.stdout.is_empty() && !printed.is_empty() {
                    c.stdout = printed;
                }
            } else {
                report.message = Some(format!("{} was killed by {name}.", lang.name()));
            }
        }
        Termination::OutputLimit => {
            if let Some(i) = in_progress {
                report.cases[i].status = CaseStatus::OutputLimit;
                report.cases[i].error = Some("Printed too much output; the program was stopped.".into());
            } else {
                report.message = Some("Printed too much output; the program was stopped.".into());
            }
        }
        Termination::Cancelled => {
            if let Some(i) = in_progress {
                report.cases[i].status = CaseStatus::Cancelled;
            }
            for c in report.cases.iter_mut().filter(|c| c.status == CaseStatus::NotRun) {
                c.status = CaseStatus::Cancelled;
            }
        }
        Termination::Exited { code } => {
            if !ended && fatal.is_none() {
                report.message = Some(format!("{} exited with code {code} before finishing.{}", lang.name(), if stderr.trim().is_empty() { String::new() } else { " See the details below.".into() }));
            }
        }
    }

    report.process = Some(ProcessInfo {
        termination: term.clone(),
        elapsed_ms: pr.elapsed_ms,
        stderr,
        sandboxed,
        guard: guard_on,
        toolchain: lang.toolchain(),
        language: lang.id().to_string(),
        command: command_line,
    });

    report.verdict = if term == Termination::Cancelled {
        Verdict::Cancelled
    } else if let Some(f) = fatal {
        report.message = Some(f);
        Verdict::RuntimeError
    } else if let Some(c) = report.cases.iter().find(|c| !matches!(c.status, CaseStatus::Passed | CaseStatus::Ran)) {
        match c.status {
            CaseStatus::Failed => Verdict::WrongAnswer,
            CaseStatus::Timeout => Verdict::TimeLimitExceeded,
            CaseStatus::Memory => Verdict::MemoryLimitExceeded,
            CaseStatus::OutputLimit => Verdict::OutputLimitExceeded,
            CaseStatus::InputError => Verdict::InvalidInput,
            CaseStatus::NotRun => {
                if report.message.is_none() {
                    report.message = Some("Some tests did not run.".into());
                }
                Verdict::RuntimeError
            }
            _ => Verdict::RuntimeError,
        }
    } else if report.message.is_some() {
        Verdict::RuntimeError
    } else if report.cases.iter().any(|c| c.status == CaseStatus::Ran) {
        Verdict::Finished
    } else {
        Verdict::Accepted
    };

    finish(report, started)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::Checker;
    use crate::java_env::JavaEnv;
    use crate::lang::{java::Java, rust::Rust};
    use crate::model::Param;
    use crate::{java_env, rust_env, support};
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct Fixture {
        _dir: tempfile::TempDir,
        java: JavaEnv,
        support: PathBuf,
        runs: PathBuf,
        checker: Mutex<Option<Checker>>,
        rust_lock: Mutex<()>,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let java = java_env::detect(None).unwrap();
        let support = support::ensure_compiled(&java, dir.path()).unwrap();
        let checker = Checker::new(java.clone(), support.clone(), dir.path().join("scratch"));
        let runs = dir.path().join("runs");
        Fixture { java, support, runs, checker: Mutex::new(Some(checker)), rust_lock: Mutex::new(()), _dir: dir }
    }

    fn func(method: &str, params: &[(&str, &str)], output_param: Option<usize>) -> Meta {
        Meta::Function {
            method: method.into(),
            params: params.iter().map(|(n, t)| Param { name: n.to_string(), ty: t.to_string() }).collect(),
            return_type: String::new(),
            output_param,
        }
    }

    fn tc(inputs: &[&str], expected: Option<&str>) -> TestCase {
        TestCase { inputs: inputs.iter().map(|s| s.to_string()).collect(), expected: expected.map(String::from) }
    }

    fn go(f: &Fixture, meta: Meta, code: &str, tests: Vec<TestCase>, timeout: u64) -> RunReport {
        let settings = Settings { timeout_secs: timeout, ..Settings::default() };
        let java = Java { env: f.java.clone(), support: f.support.clone(), checker: &f.checker };
        let env = RunEnv { lang: &java, runs_dir: &f.runs, settings: &settings };
        let req = RunRequest { slug: "t".into(), code: code.into(), meta, tests, any_order: false, language: "java".into() };
        run(&env, &req, &AtomicBool::new(false))
    }

    fn go_rust(f: &Fixture, meta: Meta, code: &str, tests: Vec<TestCase>, timeout: u64) -> RunReport {
        let settings = Settings { timeout_secs: timeout, ..Settings::default() };
        let rust = Rust { env: rust_env::detect().unwrap(), work: f.runs.join("rust-check"), lock: &f.rust_lock };
        let env = RunEnv { lang: &rust, runs_dir: &f.runs, settings: &settings };
        let req = RunRequest { slug: "t".into(), code: code.into(), meta, tests, any_order: false, language: "rust".into() };
        run(&env, &req, &AtomicBool::new(false))
    }

    const TWO_SUM: &str = "class Solution {\n    public int[] twoSum(int[] nums, int target) {\n        Map<Integer, Integer> seen = new HashMap<>();\n        for (int i = 0; i < nums.length; i++) {\n            Integer j = seen.get(target - nums[i]);\n            if (j != null) return new int[]{j, i};\n            seen.put(nums[i], i);\n        }\n        return new int[0];\n    }\n}\n";

    #[test]
    fn two_sum_accepted() {
        let f = fixture();
        let r = go(&f, func("twoSum", &[("nums", "integer[]"), ("target", "integer")], None), TWO_SUM,
            vec![tc(&["[2,7,11,15]", "9"], Some("[0,1]")), tc(&["[3,2,4]", "6"], Some("[1,2]")), tc(&["[3,3]", "6"], Some("[0,1]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");
        assert_eq!(r.summary, "3/3 passed");
        assert_eq!(r.process.as_ref().unwrap().sandboxed, sandbox::available());
    }

    #[test]
    fn wrong_answer_and_stdout() {
        let f = fixture();
        let code = "class Solution {\n  public int add(int a, int b) {\n    System.out.println(\"a=\" + a);\n    return a - b;\n  }\n}\n";
        let r = go(&f, func("add", &[("a", "integer"), ("b", "integer")], None), code, vec![tc(&["1", "1"], Some("2")), tc(&["5", "0"], Some("5"))], 10);
        assert_eq!(r.verdict, Verdict::WrongAnswer);
        assert_eq!(r.cases[0].status, CaseStatus::Failed);
        assert_eq!(r.cases[0].output.as_deref(), Some("0"));
        assert_eq!(r.cases[0].stdout, "a=1\n");
        assert_eq!(r.cases[1].status, CaseStatus::Passed);
    }

    #[test]
    fn compile_error_has_user_lines() {
        let f = fixture();
        let code = "class Solution {\n  public int f(int a) {\n    system.out.println(a);\n    return a\n  }\n}\n";
        let r = go(&f, func("f", &[("a", "integer")], None), code, vec![tc(&["1"], Some("1"))], 10);
        assert_eq!(r.verdict, Verdict::CompileError);
        let lines: Vec<i64> = r.compile.unwrap().diagnostics.iter().map(|d| d.line).collect();
        assert!(lines.contains(&4), "{lines:?}");
    }

    #[test]
    fn infinite_loop_is_time_limit_and_marks_the_test() {
        let f = fixture();
        let code = "class Solution {\n  public int f(int a) {\n    if (a == 2) { while (true) {} }\n    return a;\n  }\n}\n";
        let t0 = Instant::now();
        let r = go(&f, func("f", &[("a", "integer")], None), code, vec![tc(&["1"], Some("1")), tc(&["2"], Some("2")), tc(&["3"], Some("3"))], 2);
        assert!(t0.elapsed() < Duration::from_secs(6));
        assert_eq!(r.verdict, Verdict::TimeLimitExceeded, "{r:#?}");
        assert_eq!(r.cases[0].status, CaseStatus::Passed);
        assert_eq!(r.cases[1].status, CaseStatus::Timeout);
        assert_eq!(r.cases[2].status, CaseStatus::NotRun);
    }

    #[test]
    fn runtime_error_points_at_user_line() {
        let f = fixture();
        let code = "class Solution {\n  public int f(int[] a) {\n    int x = 0;\n    return a[5];\n  }\n}\n";
        let r = go(&f, func("f", &[("a", "integer[]")], None), code, vec![tc(&["[1,2]"], Some("1"))], 10);
        assert_eq!(r.verdict, Verdict::RuntimeError);
        assert_eq!(r.cases[0].error_line, Some(4), "{:?}", r.cases[0].trace);
        assert!(r.cases[0].error.as_ref().unwrap().contains("ArrayIndexOutOfBounds"));
    }

    #[test]
    fn dangerous_calls_are_blocked() {
        let f = fixture();
        let home = std::env::var("HOME").unwrap();
        let target = format!("{home}/sweepcode-should-not-exist.txt");
        let code = format!("class Solution {{\n  public int f(int k) throws Exception {{\n    if (k == 0) System.exit(1);\n    if (k == 1) new java.io.FileOutputStream(\"{target}\").write(1);\n    if (k == 2) Runtime.getRuntime().exec(new String[]{{\"/usr/bin/touch\", \"{target}\"}});\n    if (k == 3) new java.net.Socket(\"1.1.1.1\", 80).close();\n    return k;\n  }}\n}}\n");
        let r = go(&f, func("f", &[("k", "integer")], None), &code, vec![tc(&["0"], None), tc(&["1"], None), tc(&["2"], None), tc(&["3"], None), tc(&["4"], Some("4"))], 10);
        for i in 0..4 {
            assert!(matches!(r.cases[i].status, CaseStatus::Blocked | CaseStatus::Error), "case {i}: {:?}", r.cases[i]);
        }
        assert_eq!(r.cases[4].status, CaseStatus::Passed);
        assert!(!Path::new(&target).exists());
    }

    #[test]
    fn output_flood_is_capped_per_test() {
        let f = fixture();
        let code = "class Solution {\n  public int f(int k) {\n    for (int i = 0; i < 2_000_000; i++) System.out.println(\"spam spam spam \" + i);\n    return k;\n  }\n}\n";
        let r = go(&f, func("f", &[("k", "integer")], None), code, vec![tc(&["1"], Some("1"))], 20);
        assert_eq!(r.verdict, Verdict::Accepted, "{:?}", r.message);
        assert!(r.cases[0].stdout_truncated);
        assert!(r.cases[0].stdout.len() <= 64 * 1024 + 16);
    }

    #[test]
    fn lists_trees_void_and_design() {
        let f = fixture();
        let add = "class Solution {\n  public ListNode addTwoNumbers(ListNode l1, ListNode l2) {\n    ListNode d = new ListNode(0), c = d; int carry = 0;\n    while (l1 != null || l2 != null || carry > 0) {\n      int s = carry + (l1 == null ? 0 : l1.val) + (l2 == null ? 0 : l2.val);\n      carry = s / 10; c.next = new ListNode(s % 10); c = c.next;\n      if (l1 != null) l1 = l1.next; if (l2 != null) l2 = l2.next;\n    }\n    return d.next;\n  }\n}\n";
        let r = go(&f, func("addTwoNumbers", &[("l1", "ListNode"), ("l2", "ListNode")], None), add,
            vec![tc(&["[2,4,3]", "[5,6,4]"], Some("[7,0,8]")), tc(&["[9,9,9,9,9,9,9]", "[9,9,9,9]"], Some("[8,9,9,9,0,0,0,1]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let inv = "class Solution {\n  public TreeNode invertTree(TreeNode r) {\n    if (r == null) return null;\n    TreeNode t = r.left; r.left = invertTree(r.right); r.right = invertTree(t);\n    return r;\n  }\n}\n";
        let r = go(&f, func("invertTree", &[("root", "TreeNode")], None), inv,
            vec![tc(&["[4,2,7,1,3,6,9]"], Some("[4,7,2,9,6,3,1]")), tc(&["[]"], Some("[]")), tc(&["[1,null,2]"], Some("[1,2]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let rot = "class Solution {\n  public void rotate(int[] nums, int k) {\n    int n = nums.length; k %= n; int[] c = nums.clone();\n    for (int i = 0; i < n; i++) nums[(i + k) % n] = c[i];\n  }\n}\n";
        let r = go(&f, func("rotate", &[("nums", "integer[]"), ("k", "integer")], Some(0)), rot,
            vec![tc(&["[1,2,3,4,5,6,7]", "3"], Some("[5,6,7,1,2,3,4]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let med = "class Solution {\n  public double findMedianSortedArrays(int[] a, int[] b) {\n    int[] m = new int[a.length + b.length]; int k = 0;\n    for (int x : a) m[k++] = x; for (int x : b) m[k++] = x; Arrays.sort(m);\n    int n = m.length; return n % 2 == 1 ? m[n/2] : (m[n/2-1] + m[n/2]) / 2.0;\n  }\n}\n";
        let r = go(&f, func("findMedianSortedArrays", &[("nums1", "integer[]"), ("nums2", "integer[]")], None), med,
            vec![tc(&["[1,3]", "[2]"], Some("2.00000")), tc(&["[1,2]", "[3,4]"], Some("2.50000"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let lru = "class LRUCache {\n  private final LinkedHashMap<Integer,Integer> m; private final int cap;\n  public LRUCache(int capacity) { cap = capacity; m = new LinkedHashMap<>(16, 0.75f, true); }\n  public int get(int key) { return m.getOrDefault(key, -1); }\n  public void put(int key, int value) { m.put(key, value); if (m.size() > cap) m.remove(m.keySet().iterator().next()); }\n}\n";
        let meta = Meta::Design { class_name: "LRUCache".into(), constructor_params: vec![], methods: vec![] };
        let r = go(&f, meta, lru, vec![tc(&[
            "[\"LRUCache\",\"put\",\"put\",\"get\",\"put\",\"get\",\"put\",\"get\",\"get\",\"get\"]",
            "[[2],[1,1],[2,2],[1],[3,3],[2],[4,4],[1],[3],[4]]",
        ], Some("[null, null, null, 1, null, -1, null, -1, 3, 4]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let strs = "class Solution {\n  public List<List<String>> groupAnagrams(String[] strs) {\n    Map<String, List<String>> m = new TreeMap<>();\n    for (String s : strs) { char[] c = s.toCharArray(); Arrays.sort(c); m.computeIfAbsent(new String(c), x -> new ArrayList<>()).add(s); }\n    return new ArrayList<>(m.values());\n  }\n}\n";
        let r = go(&f, func("groupAnagrams", &[("strs", "string[]")], None), strs, vec![tc(&["[\"eat\",\"tea\",\"tan\",\"ate\",\"nat\",\"bat\"]"], None)], 10);
        assert_eq!(r.verdict, Verdict::Finished);
        assert_eq!(r.cases[0].output.as_deref(), Some("[[\"bat\"],[\"eat\",\"tea\",\"ate\"],[\"tan\",\"nat\"]]"));

        let chars = "class Solution {\n  public boolean f(char[][] b, List<List<Integer>> g, long x, String s) {\n    return b[0][1] == '.' && g.get(1).get(0) == 3 && x == 10000000000L && s.equals(\"hi\\\"\");\n  }\n}\n";
        let r = go(&f, func("f", &[("b", "character[][]"), ("g", "list<list<integer>>"), ("x", "long"), ("s", "string")], None), chars,
            vec![tc(&["[[\"5\",\".\"]]", "[[1,2],[3]]", "10000000000", "\"hi\\\"\""], Some("true"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");
    }

    #[test]
    fn bad_test_input_and_missing_method() {
        let f = fixture();
        let r = go(&f, func("twoSum", &[("nums", "integer[]"), ("target", "integer")], None), TWO_SUM, vec![tc(&["[2,7", "9"], Some("[0,1]"))], 10);
        assert_eq!(r.verdict, Verdict::InvalidInput);
        assert!(r.cases[0].error.as_ref().unwrap().contains("nums"), "{:?}", r.cases[0].error);

        let r = go(&f, func("threeSum", &[("nums", "integer[]")], None), TWO_SUM, vec![tc(&["[1]"], None)], 10);
        assert_eq!(r.verdict, Verdict::RuntimeError);
        assert!(r.message.as_ref().unwrap().contains("threeSum"));

        let r = go(&f, func("twoSum", &[("nums", "integer[]"), ("target", "integer")], None), TWO_SUM, vec![tc(&["[1]"], None)], 10);
        assert_eq!(r.verdict, Verdict::InvalidInput);
    }

    #[test]
    fn memory_and_stack() {
        let f = fixture();
        let code = "class Solution {\n  public int f(int k) {\n    if (k == 1) { List<long[]> l = new ArrayList<>(); while (true) l.add(new long[1 << 20]); }\n    return f(k + 2);\n  }\n}\n";
        let r = go(&f, func("f", &[("k", "integer")], None), code, vec![tc(&["1"], None), tc(&["2"], None)], 20);
        assert_eq!(r.cases[0].status, CaseStatus::Memory);
        assert_eq!(r.verdict, Verdict::MemoryLimitExceeded);
        assert_eq!(r.cases[1].status, CaseStatus::Error);
        assert_eq!(r.cases[1].error_kind.as_deref(), Some("stackoverflow"));
    }

    /// Follows docs/TUTORIAL.md "Add your own problem" exactly, so the docs can't rot.
    #[test]
    fn tutorial_custom_problem_example() {
        let f = fixture();
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/custom-problem/count-target");
        let data = tempfile::tempdir().unwrap();
        let dest = data.path().join("problems/count-target");
        std::fs::create_dir_all(&dest).unwrap();
        for name in ["problem.json", "tests.json", "Solution.java"] {
            std::fs::copy(example.join(name), dest.join(name)).unwrap();
        }
        let store = crate::store::Store::new(data.path().to_path_buf()).unwrap();
        let listed: Vec<String> = store.list().into_iter().map(|p| p.title).collect();
        assert_eq!(listed, ["Count Target"]);
        let ws = store.load("count-target").unwrap();
        assert_eq!(ws.tests.len(), 4);
        assert!(ws.codes["java"].contains("countTarget"));

        let solved = "class Solution {\n    public int countTarget(int[] nums, int target) {\n        int n = 0;\n        for (int x : nums) if (x == target) n++;\n        return n;\n    }\n}\n";
        let r = go(&f, ws.problem.meta.clone().unwrap(), solved, ws.tests.clone(), 10);
        // Three tests have an expected value, the last one only shows its output.
        assert_eq!(r.verdict, Verdict::Finished, "{r:#?}");
        assert_eq!(r.summary, "3/3 passed");
        assert_eq!(r.cases[3].status, CaseStatus::Ran);
        assert_eq!(r.cases[3].output.as_deref(), Some("2"));

        // The untouched starter code is a compile error, as the tutorial says.
        let r = go(&f, ws.problem.meta.clone().unwrap(), &ws.codes["java"], ws.tests.clone(), 10);
        assert_eq!(r.verdict, Verdict::CompileError);
    }

    // ---------------------------------------------------------------- Rust

    const RUST_TWO_SUM: &str = "impl Solution {\n    pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {\n        let mut seen = HashMap::new();\n        for (i, &x) in nums.iter().enumerate() {\n            if let Some(&j) = seen.get(&(target - x)) {\n                return vec![j as i32, i as i32];\n            }\n            seen.insert(x, i);\n        }\n        vec![]\n    }\n}\n";

    #[test]
    fn rust_two_sum_accepted_and_sandboxed() {
        let f = fixture();
        let r = go_rust(&f, func("twoSum", &[("nums", "integer[]"), ("target", "integer")], None), RUST_TWO_SUM,
            vec![tc(&["[2,7,11,15]", "9"], Some("[0,1]")), tc(&["[3,2,4]", "6"], Some("[1,2]")), tc(&["[3,3]", "6"], Some("[0,1]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");
        let p = r.process.unwrap();
        assert_eq!(p.sandboxed, sandbox::available());
        assert_eq!(p.language, "rust");
        assert!(p.toolchain.starts_with("rustc "));
    }

    #[test]
    fn rust_wrong_answer_stdout_and_compile_errors() {
        let f = fixture();
        let code = "impl Solution {\n    pub fn add(a: i32, b: i32) -> i32 {\n        println!(\"a={}\", a);\n        a - b\n    }\n}\n";
        let r = go_rust(&f, func("add", &[("a", "integer"), ("b", "integer")], None), code, vec![tc(&["1", "1"], Some("2")), tc(&["5", "0"], Some("5"))], 10);
        assert_eq!(r.verdict, Verdict::WrongAnswer, "{r:#?}");
        assert_eq!(r.cases[0].output.as_deref(), Some("0"));
        assert_eq!(r.cases[0].stdout, "a=1\n");
        assert_eq!(r.cases[1].status, CaseStatus::Passed);

        let bad = "impl Solution {\n    pub fn add(a: i32, b: i32) -> i32 {\n        let s: String = a;\n        a + b\n    }\n}\n";
        let r = go_rust(&f, func("add", &[("a", "integer"), ("b", "integer")], None), bad, vec![tc(&["1", "1"], Some("2"))], 10);
        assert_eq!(r.verdict, Verdict::CompileError, "{r:#?}");
        assert_eq!(r.compile.unwrap().diagnostics[0].line, 3);

        // Untouched LeetCode starter code: the empty body doesn't return a value.
        let starter = "impl Solution {\n    pub fn add(a: i32, b: i32) -> i32 {\n        \n    }\n}";
        let r = go_rust(&f, func("add", &[("a", "integer"), ("b", "integer")], None), starter, vec![tc(&["1", "1"], None)], 10);
        assert_eq!(r.verdict, Verdict::CompileError);
    }

    #[test]
    fn rust_panic_line_and_infinite_loop() {
        let f = fixture();
        let code = "impl Solution {\n    pub fn f(a: Vec<i32>) -> i32 {\n        if a.len() == 3 { loop {} }\n        a[5]\n    }\n}\n";
        let r = go_rust(&f, func("f", &[("a", "integer[]")], None), code, vec![tc(&["[1,2]"], Some("1")), tc(&["[1,2,3]"], None), tc(&["[1]"], None)], 2);
        assert_eq!(r.verdict, Verdict::RuntimeError, "{r:#?}");
        assert_eq!(r.cases[0].error_line, Some(4), "{:?}", r.cases[0]);
        assert!(r.cases[0].error.as_ref().unwrap().contains("index out of bounds"));
        assert_eq!(r.cases[1].status, CaseStatus::Timeout);
        assert_eq!(r.cases[2].status, CaseStatus::NotRun);
    }

    #[test]
    fn rust_lists_trees_in_place_design_and_types() {
        let f = fixture();
        let add = "impl Solution {\n    pub fn add_two_numbers(l1: Option<Box<ListNode>>, l2: Option<Box<ListNode>>) -> Option<Box<ListNode>> {\n        let (mut a, mut b, mut carry) = (l1, l2, 0);\n        let mut dummy = Box::new(ListNode::new(0));\n        let mut tail = &mut dummy;\n        while a.is_some() || b.is_some() || carry > 0 {\n            let mut s = carry;\n            if let Some(n) = a { s += n.val; a = n.next; }\n            if let Some(n) = b { s += n.val; b = n.next; }\n            carry = s / 10;\n            tail.next = Some(Box::new(ListNode::new(s % 10)));\n            tail = tail.next.as_mut().unwrap();\n        }\n        dummy.next\n    }\n}\n";
        let r = go_rust(&f, func("addTwoNumbers", &[("l1", "ListNode"), ("l2", "ListNode")], None), add,
            vec![tc(&["[2,4,3]", "[5,6,4]"], Some("[7,0,8]")), tc(&["[9,9,9,9,9,9,9]", "[9,9,9,9]"], Some("[8,9,9,9,0,0,0,1]")), tc(&["[0]", "[0]"], Some("[0]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let inv = "use std::rc::Rc;\nuse std::cell::RefCell;\nimpl Solution {\n    pub fn invert_tree(root: Option<Rc<RefCell<TreeNode>>>) -> Option<Rc<RefCell<TreeNode>>> {\n        if let Some(n) = root.clone() {\n            let mut b = n.borrow_mut();\n            let l = b.left.take();\n            let r = b.right.take();\n            b.left = Self::invert_tree(r);\n            b.right = Self::invert_tree(l);\n        }\n        root\n    }\n}\n";
        let r = go_rust(&f, func("invertTree", &[("root", "TreeNode")], None), inv,
            vec![tc(&["[4,2,7,1,3,6,9]"], Some("[4,7,2,9,6,3,1]")), tc(&["[]"], Some("[]")), tc(&["[1,null,2]"], Some("[1,2]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let rot = "impl Solution {\n    pub fn rotate(nums: &mut Vec<i32>, k: i32) {\n        let n = nums.len();\n        let k = k as usize % n;\n        nums.rotate_right(k);\n    }\n}\n";
        let r = go_rust(&f, func("rotate", &[("nums", "integer[]"), ("k", "integer")], Some(0)), rot,
            vec![tc(&["[1,2,3,4,5,6,7]", "3"], Some("[5,6,7,1,2,3,4]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let med = "impl Solution {\n    pub fn find_median_sorted_arrays(nums1: Vec<i32>, nums2: Vec<i32>) -> f64 {\n        let mut m = [nums1, nums2].concat();\n        m.sort();\n        let n = m.len();\n        if n % 2 == 1 { m[n / 2] as f64 } else { (m[n / 2 - 1] + m[n / 2]) as f64 / 2.0 }\n    }\n}\n";
        let r = go_rust(&f, func("findMedianSortedArrays", &[("nums1", "integer[]"), ("nums2", "integer[]")], None), med,
            vec![tc(&["[1,3]", "[2]"], Some("2.00000")), tc(&["[1,2]", "[3,4]"], Some("2.50000"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let lru = "use std::collections::HashMap;\nstruct LRUCache { cap: usize, tick: u64, m: HashMap<i32, (i32, u64)> }\nimpl LRUCache {\n    fn new(capacity: i32) -> Self { LRUCache { cap: capacity as usize, tick: 0, m: HashMap::new() } }\n    fn get(&mut self, key: i32) -> i32 {\n        self.tick += 1;\n        let t = self.tick;\n        match self.m.get_mut(&key) { Some(e) => { e.1 = t; e.0 } None => -1 }\n    }\n    fn put(&mut self, key: i32, value: i32) {\n        self.tick += 1;\n        if !self.m.contains_key(&key) && self.m.len() == self.cap {\n            let old = *self.m.iter().min_by_key(|(_, v)| v.1).unwrap().0;\n            self.m.remove(&old);\n        }\n        self.m.insert(key, (value, self.tick));\n    }\n}\n";
        let meta = Meta::Design { class_name: "LRUCache".into(), constructor_params: vec![Param { name: "capacity".into(), ty: "integer".into() }], methods: vec!["get".into(), "put".into()] };
        let r = go_rust(&f, meta, lru, vec![tc(&[
            "[\"LRUCache\",\"put\",\"put\",\"get\",\"put\",\"get\",\"put\",\"get\",\"get\",\"get\"]",
            "[[2],[1,1],[2,2],[1],[3,3],[2],[4,4],[1],[3],[4]]",
        ], Some("[null, null, null, 1, null, -1, null, -1, 3, 4]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");

        let mix = "impl Solution {\n    pub fn f(board: Vec<Vec<char>>, words: Vec<String>, big: i64, s: String) -> Vec<Vec<String>> {\n        assert_eq!(board[0][1], '.');\n        assert_eq!(big, 10000000000);\n        vec![words, vec![s]]\n    }\n}\n";
        let r = go_rust(&f, func("f", &[("board", "character[][]"), ("words", "string[]"), ("big", "long"), ("s", "string")], None), mix,
            vec![tc(&["[[\"5\",\".\"]]", "[\"eat\",\"tea\"]", "10000000000", "\"hi\\\"\""], Some("[[\"eat\",\"tea\"],[\"hi\\\"\"]]"))], 10);
        assert_eq!(r.verdict, Verdict::Accepted, "{r:#?}");
    }

    #[test]
    fn rust_sandbox_memory_stack_and_bad_input() {
        let f = fixture();
        let home = std::env::var("HOME").unwrap();
        let target = format!("{home}/sweepcode-rust-should-not-exist.txt");
        let code = format!("impl Solution {{\n    pub fn f(k: i32) -> i32 {{\n        if k == 0 {{ std::fs::write(\"{target}\", \"x\").unwrap(); }}\n        if k == 1 {{ std::process::Command::new(\"/usr/bin/touch\").arg(\"{target}\").status().unwrap(); }}\n        if k == 2 {{ std::net::TcpStream::connect(\"1.1.1.1:80\").unwrap(); }}\n        k\n    }}\n}}\n");
        let r = go_rust(&f, func("f", &[("k", "integer")], None), &code, vec![tc(&["0"], None), tc(&["1"], None), tc(&["2"], None), tc(&["3"], Some("3"))], 10);
        for i in 0..3 {
            assert!(matches!(r.cases[i].status, CaseStatus::Blocked | CaseStatus::Error), "case {i}: {:?}", r.cases[i]);
        }
        assert_eq!(r.cases[3].status, CaseStatus::Passed);
        assert!(!Path::new(&target).exists());

        let mem = "impl Solution {\n    pub fn f(k: i32) -> i32 {\n        let mut v: Vec<Vec<u8>> = Vec::new();\n        loop { v.push(vec![1u8; 1 << 20]); }\n    }\n}\n";
        let r = go_rust(&f, func("f", &[("k", "integer")], None), mem, vec![tc(&["1"], None), tc(&["2"], None)], 20);
        assert_eq!(r.verdict, Verdict::MemoryLimitExceeded, "{r:#?}");
        assert_eq!(r.cases[0].status, CaseStatus::Memory);

        let deep = "impl Solution {\n    pub fn f(k: i32) -> i32 {\n        if k == 0 { 0 } else { 1 + Self::f(k - 1) }\n    }\n}\n";
        let r = go_rust(&f, func("f", &[("k", "integer")], None), deep, vec![tc(&["100000"], Some("100000")), tc(&["2000000000"], None)], 20);
        assert_eq!(r.cases[0].status, CaseStatus::Passed, "{:?}", r.cases[0]);
        assert_eq!(r.cases[1].status, CaseStatus::Error);
        assert!(r.cases[1].error.as_ref().unwrap().contains("Stack overflow"), "{:?}", r.cases[1]);

        let r = go_rust(&f, func("twoSum", &[("nums", "integer[]"), ("target", "integer")], None), RUST_TWO_SUM, vec![tc(&["[2,7", "9"], None)], 10);
        assert_eq!(r.verdict, Verdict::InvalidInput);
        assert!(r.cases[0].error.as_ref().unwrap().contains("nums"));

        let r = go_rust(&f, func("threeSum", &[("nums", "integer[]")], None), RUST_TWO_SUM, vec![tc(&["[1]"], None)], 10);
        assert_eq!(r.verdict, Verdict::InternalError);
        assert!(r.message.as_ref().unwrap().contains("three_sum"), "{:?}", r.message);
    }

    #[test]
    fn prune_keeps_newest() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..25 {
            std::fs::create_dir_all(d.path().join(format!("20260101-0000{i:02}-000"))).unwrap();
        }
        prune_runs(d.path(), 20);
        let n = std::fs::read_dir(d.path()).unwrap().count();
        assert_eq!(n, 20);
        assert!(d.path().join("20260101-000024-000").exists());
        assert!(!d.path().join("20260101-000000-000").exists());
    }
}
