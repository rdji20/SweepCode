pub mod checker;
pub mod java_env;
pub mod judge;
pub mod leetcode;
pub mod model;
pub mod proc;
pub mod runner;
pub mod sample;
pub mod sandbox;
pub mod store;
pub mod support;

use checker::{Checker, CompileResult};
use java_env::JavaEnv;
use model::{ProblemSummary, TestCase};
use runner::{RunEnv, RunReport, RunRequest};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use store::{Settings, StoredSummary, Store, Workspace};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};
use tauri_plugin_opener::OpenerExt;

const LOG_FILE: &str = "sweepcode";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub runs: PathBuf,
}

/// Everything the commands share. Lives behind an Arc so blocking work can
/// move to a worker thread without holding up the UI.
pub struct Inner {
    pub paths: Paths,
    pub store: Store,
    pub java: Mutex<Result<JavaEnv, String>>,
    pub support: Mutex<Option<PathBuf>>,
    pub checker: Mutex<Option<Checker>>,
    pub compiler_ready: AtomicBool,
    pub run_lock: Mutex<()>,
    pub cancel: Mutex<Option<Arc<AtomicBool>>>,
}

type AppState<'a> = State<'a, Arc<Inner>>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvInfo {
    pub java: Option<JavaEnv>,
    pub java_error: Option<String>,
    pub compiler_ready: bool,
    pub sandbox: bool,
    pub java_guard: bool,
    pub paths: Paths,
    pub app_version: String,
}

fn env_info(inner: &Inner) -> EnvInfo {
    let java = inner.java.lock().unwrap().clone();
    EnvInfo {
        java_guard: java.as_ref().map(|j| j.supports_security_manager()).unwrap_or(false),
        java: java.as_ref().ok().cloned(),
        java_error: java.err(),
        compiler_ready: inner.compiler_ready.load(Ordering::SeqCst),
        sandbox: sandbox::available(),
        paths: inner.paths.clone(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Finds the JDK, compiles helper classes and starts the compiler process.
/// Runs on a background thread at startup and whenever the JDK setting changes.
fn init_java(app: &AppHandle, inner: &Arc<Inner>) {
    inner.compiler_ready.store(false, Ordering::SeqCst);
    *inner.checker.lock().unwrap() = None;
    let settings = inner.store.settings();
    let override_home = settings.java_home.as_ref().map(PathBuf::from);
    let result = java_env::detect(override_home.as_deref()).and_then(|java| {
        log::info!("using Java {} at {} (found via {})", java.version, java.home.display(), java.source);
        let support = support::ensure_compiled(&java, &inner.paths.cache)?;
        support::prune(&inner.paths.cache, &support);
        let mut checker = Checker::new(java.clone(), support.clone(), inner.paths.cache.join("scratch"));
        checker.warm_up()?;
        // First compile loads javac's classes; doing it now makes the user's first check fast.
        let _ = checker.compile("Solution.java", "class Solution {}", None, Duration::from_secs(60));
        *inner.support.lock().unwrap() = Some(support);
        *inner.checker.lock().unwrap() = Some(checker);
        Ok(java)
    });
    match &result {
        Ok(_) => {
            inner.compiler_ready.store(true, Ordering::SeqCst);
            log::info!("compiler ready; sandbox-exec available: {}", sandbox::available());
        }
        Err(e) => log::error!("Java setup failed: {e}"),
    }
    *inner.java.lock().unwrap() = result;
    let _ = app.emit("env-changed", env_info(inner));
}

fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> impl std::future::Future<Output = Result<T, String>> {
    async move { tauri::async_runtime::spawn_blocking(f).await.map_err(|e| format!("internal error: {e}"))? }
}

// ------------------------------------------------------------------ commands

#[tauri::command]
fn get_env(state: AppState) -> EnvInfo {
    env_info(&state)
}

#[tauri::command]
fn list_problems(state: AppState) -> Vec<StoredSummary> {
    state.store.list()
}

#[tauri::command]
async fn open_problem(state: AppState<'_>, slug: String) -> Result<Workspace, String> {
    let inner = state.inner().clone();
    blocking(move || {
        inner.store.touch(&slug)?;
        inner.store.load(&slug)
    })
    .await
}

#[tauri::command]
async fn fetch_problem(state: AppState<'_>, input: String) -> Result<Workspace, String> {
    log::info!("fetching problem for '{input}'");
    let problem = leetcode::resolve(&input).await.map_err(|e| {
        log::warn!("fetch '{input}' failed: {e}");
        e
    })?;
    log::info!("fetched {} '{}' ({} examples, java={})", problem.slug, problem.title, problem.examples.len(), problem.java_code.is_some());
    let inner = state.inner().clone();
    blocking(move || {
        inner.store.save_problem(&problem)?;
        inner.store.touch(&problem.slug)?;
        inner.store.load(&problem.slug)
    })
    .await
}

#[tauri::command]
async fn search_problems(query: String) -> Result<Vec<ProblemSummary>, String> {
    let q = query.trim().to_string();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let q = match leetcode::parse_lookup(&q) {
        leetcode::Lookup::Slug(s) => s.replace('-', " "),
        leetcode::Lookup::Number(n) => n.to_string(),
        leetcode::Lookup::Query(q) => q,
    };
    leetcode::search(&q, 12).await.map_err(|e| {
        log::warn!("search '{q}' failed: {e}");
        e
    })
}

#[tauri::command]
fn delete_problem(state: AppState, slug: String) -> Result<(), String> {
    log::info!("removing problem {slug} (moved to trash)");
    state.store.delete(&slug)
}

#[tauri::command]
fn save_code(state: AppState, slug: String, code: String) -> Result<(), String> {
    state.store.save_code(&slug, &code)
}

#[tauri::command]
fn save_tests(state: AppState, slug: String, tests: Vec<TestCase>) -> Result<(), String> {
    state.store.save_tests(&slug, &tests)
}

#[tauri::command]
async fn check_code(state: AppState<'_>, file_name: String, code: String) -> Result<CompileResult, String> {
    let inner = state.inner().clone();
    blocking(move || {
        if !inner.compiler_ready.load(Ordering::SeqCst) {
            return Err("compiler not ready".into());
        }
        let mut guard = inner.checker.lock().unwrap_or_else(|p| p.into_inner());
        let checker = guard.as_mut().ok_or("compiler not ready")?;
        checker.compile(&file_name, &code, None, Duration::from_secs(20)).map_err(|e| {
            log::warn!("live check failed: {e}");
            e
        })
    })
    .await
}

#[tauri::command]
async fn run_code(state: AppState<'_>, request: RunRequest) -> Result<RunReport, String> {
    let inner = state.inner().clone();
    // A new run stops the previous one.
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(prev) = inner.cancel.lock().unwrap().replace(cancel.clone()) {
        prev.store(true, Ordering::SeqCst);
    }
    blocking(move || {
        let _one_at_a_time = inner.run_lock.lock().unwrap_or_else(|p| p.into_inner());
        let java = inner.java.lock().unwrap().clone()?;
        let support = inner.support.lock().unwrap().clone().ok_or("helper classes are not compiled yet")?;
        let settings = inner.store.settings();
        let env = RunEnv { java: &java, support: &support, runs_dir: &inner.paths.runs, settings: &settings, checker: &inner.checker };
        let _ = inner.store.save_code(&request.slug, &request.code);
        let report = runner::run(&env, &request, &cancel);
        runner::prune_runs(&inner.paths.runs, runner::KEEP_RUNS);
        let _ = inner.store.set_verdict(&request.slug, &report.verdict.key());
        Ok(report)
    })
    .await
}

#[tauri::command]
fn cancel_run(state: AppState) {
    if let Some(c) = state.cancel.lock().unwrap().as_ref() {
        log::info!("run cancelled by user");
        c.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
fn get_settings(state: AppState) -> Settings {
    state.store.settings()
}

#[tauri::command]
fn save_settings(app: AppHandle, state: AppState, settings: Settings) -> Result<Settings, String> {
    let before = state.store.settings();
    let saved = state.store.save_settings(&settings)?;
    log::info!("settings saved: {saved:?}");
    if before.java_home != saved.java_home {
        let inner = state.inner().clone();
        std::thread::spawn(move || init_java(&app, &inner));
    }
    Ok(saved)
}

#[tauri::command]
fn retry_java(app: AppHandle, state: AppState) {
    let inner = state.inner().clone();
    std::thread::spawn(move || init_java(&app, &inner));
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogTail {
    path: String,
    lines: Vec<String>,
}

#[tauri::command]
fn read_logs(state: AppState, max_lines: usize) -> LogTail {
    let path = state.paths.logs.join(format!("{LOG_FILE}.log"));
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(max_lines.clamp(10, 5000));
    LogTail { path: path.display().to_string(), lines: lines[start..].iter().map(|s| s.to_string()).collect() }
}

#[tauri::command]
fn reveal(app: AppHandle, state: AppState, which: String) -> Result<(), String> {
    let path = match which.as_str() {
        "logs" => state.paths.logs.clone(),
        "runs" => state.paths.runs.clone(),
        "data" => state.paths.data.clone(),
        other => {
            let id = other.strip_prefix("run:").ok_or("unknown folder")?;
            if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err("bad run id".into());
            }
            state.paths.runs.join(id)
        }
    };
    std::fs::create_dir_all(&path).ok();
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_leetcode(app: AppHandle, slug: String) -> Result<(), String> {
    if !leetcode::valid_slug(&slug) {
        return Err("bad slug".into());
    }
    app.opener().open_url(format!("https://leetcode.com/problems/{slug}/"), None::<&str>).map_err(|e| e.to_string())
}

// --------------------------------------------------------------------- setup

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_plugin = tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([
            Target::new(TargetKind::LogDir { file_name: Some(LOG_FILE.into()) }),
            Target::new(TargetKind::Stdout),
        ])
        .level(log::LevelFilter::Info)
        .level_for("sweepcode_lib", log::LevelFilter::Debug)
        .max_file_size(2_000_000)
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build();

    tauri::Builder::default()
        .plugin(log_plugin)
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            std::panic::set_hook(Box::new(|info| {
                log::error!("PANIC: {info}");
            }));
            let path = app.path();
            let paths = Paths {
                data: path.app_data_dir()?,
                cache: path.app_cache_dir()?,
                logs: path.app_log_dir()?,
                runs: path.app_cache_dir()?.join("runs"),
            };
            // Renamed from prob-warp: bring the old data folder over before anything is created.
            if let Some(parent) = paths.data.parent() {
                match store::migrate_legacy_data(&parent.join("com.probwarp.app"), &paths.data) {
                    Ok(true) => log::info!("moved data from com.probwarp.app to {}", paths.data.display()),
                    Ok(false) => {}
                    Err(e) => log::warn!("could not move old prob-warp data: {e}"),
                }
            }
            for p in [&paths.data, &paths.cache, &paths.logs, &paths.runs] {
                std::fs::create_dir_all(p)?;
            }
            log::info!("SweepCode {} starting; data={} cache={} logs={}", env!("CARGO_PKG_VERSION"), paths.data.display(), paths.cache.display(), paths.logs.display());
            let store = Store::new(paths.data.clone())?;
            if store.is_empty() {
                store.save_problem(&sample::two_sum())?;
                log::info!("added the offline Two Sum sample");
            }
            let inner = Arc::new(Inner {
                paths,
                store,
                java: Mutex::new(Err("looking for Java...".into())),
                support: Mutex::new(None),
                checker: Mutex::new(None),
                compiler_ready: AtomicBool::new(false),
                run_lock: Mutex::new(()),
                cancel: Mutex::new(None),
            });
            app.manage(inner.clone());
            let handle = app.handle().clone();
            std::thread::spawn(move || init_java(&handle, &inner));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_env,
            list_problems,
            open_problem,
            fetch_problem,
            search_problems,
            delete_problem,
            save_code,
            save_tests,
            check_code,
            run_code,
            cancel_run,
            get_settings,
            save_settings,
            retry_java,
            read_logs,
            reveal,
            open_leetcode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
