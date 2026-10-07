//! One interface for every language the app can run.
//!
//! The runner, sandbox, judge, storage and UI are shared. Each language only
//! answers these questions: how do I check code, how do I build it for a run,
//! what do I start, and where in the user's file did an error happen.

pub mod java;
pub mod rust;
pub mod rust_codegen;

use crate::checker::CompileResult;
use crate::model::Meta;
use crate::runner::RunRequest;
use crate::store::Settings;
use std::path::{Path, PathBuf};

/// Result of building the user's code for a run.
pub enum Build {
    /// Compiled. Start `program args...` inside the sandbox; `exec_dir` is the
    /// only folder the sandbox lets it execute programs from.
    Ready { compile: CompileResult, program: PathBuf, args: Vec<String>, exec_dir: PathBuf },
    /// The user's code doesn't compile; show these errors.
    Failed(CompileResult),
    /// Something outside the user's code went wrong (missing toolchain,
    /// a method signature the app can't call yet, ...).
    Fatal(String),
}

pub trait Language {
    /// "java" or "rust"; also the key used for saved code.
    fn id(&self) -> &'static str;
    /// "Java" or "Rust", for messages.
    fn name(&self) -> &'static str;
    /// "Java 17.0.8", "rustc 1.96.0".
    fn toolchain(&self) -> String;
    /// The user's code file, e.g. "Solution.java" or "solution.rs".
    fn file_name(&self, meta: &Meta) -> String;
    /// Compile only, for the live red underlines.
    fn check(&self, file_name: &str, code: &str) -> Result<CompileResult, String>;
    /// Compile into `run_dir` for a run. `job.txt` is already written there.
    fn build(&self, req: &RunRequest, run_dir: &Path, settings: &Settings) -> Build;
    /// Rewrites error locations to the user's line numbers; returns the first one.
    fn map_trace(&self, trace: &[String], file_name: &str) -> (Vec<String>, Option<i64>);
}

/// Language ids the app understands.
pub fn known(id: &str) -> bool {
    matches!(id, "java" | "rust")
}
