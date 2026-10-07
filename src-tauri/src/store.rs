//! Saves problems, test cases, code and settings as plain files so they are
//! easy to inspect:
//!
//!   <data>/problems/<slug>/problem.json   problem from LeetCode
//!   <data>/problems/<slug>/tests.json     editable test cases
//!   <data>/problems/<slug>/<File>.java    your code
//!   <data>/problems/<slug>/state.json     last opened, last verdict
//!   <data>/settings.json

use crate::leetcode::valid_slug;
use crate::model::{Problem, TestCase};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub timeout_secs: u64,
    pub memory_mb: u32,
    pub font_size: u32,
    pub check_delay_ms: u32,
    /// Optional JDK folder chosen by the user (the one containing bin/java).
    pub java_home: Option<String>,
    /// Declaration templates (Tab after `map`, `list`, ...).
    pub templates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { timeout_secs: 10, memory_mb: 256, font_size: 14, check_delay_ms: 450, java_home: None, templates: true }
    }
}

impl Settings {
    /// Keeps values inside sane bounds no matter what the file says.
    pub fn clamped(mut self) -> Self {
        self.timeout_secs = self.timeout_secs.clamp(1, 60);
        self.memory_mb = self.memory_mb.clamp(64, 2048);
        self.font_size = self.font_size.clamp(10, 28);
        self.check_delay_ms = self.check_delay_ms.clamp(150, 3000);
        self.java_home = self.java_home.filter(|s| !s.trim().is_empty());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProblemState {
    /// When the problem was first added; the sidebar is ordered by this and never reshuffles.
    pub added_at: i64,
    pub opened_at: i64,
    pub last_verdict: Option<String>,
    pub solved: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSummary {
    pub slug: String,
    pub id: String,
    pub title: String,
    pub difficulty: String,
    pub added_at: i64,
    pub opened_at: i64,
    pub last_verdict: Option<String>,
    pub solved: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub problem: Problem,
    pub tests: Vec<TestCase>,
    pub code: String,
    pub state: ProblemState,
}

pub struct Store {
    root: PathBuf,
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("cannot save {}: {e}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} is corrupted: {e}", path.display()))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Store {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(root.join("problems")).map_err(|e| format!("cannot create {}: {e}", root.display()))?;
        Ok(Store { root })
    }

    fn dir(&self, slug: &str) -> Result<PathBuf, String> {
        if !valid_slug(slug) {
            return Err(format!("invalid problem id '{slug}'"));
        }
        Ok(self.root.join("problems").join(slug))
    }

    fn code_path(&self, slug: &str, problem: &Problem) -> Result<PathBuf, String> {
        let name = problem.meta.as_ref().map(|m| m.file_name()).unwrap_or_else(|| "Solution.java".into());
        Ok(self.dir(slug)?.join(name))
    }

    pub fn settings(&self) -> Settings {
        read_json::<Settings>(&self.root.join("settings.json")).unwrap_or_default().clamped()
    }

    pub fn save_settings(&self, s: &Settings) -> Result<Settings, String> {
        let s = s.clone().clamped();
        write_atomic(&self.root.join("settings.json"), serde_json::to_string_pretty(&s).unwrap().as_bytes())?;
        Ok(s)
    }

    /// Saves a freshly fetched problem. Keeps existing code and tests.
    pub fn save_problem(&self, p: &Problem) -> Result<(), String> {
        let dir = self.dir(&p.slug)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        write_atomic(&dir.join("problem.json"), serde_json::to_string_pretty(p).unwrap().as_bytes())?;
        if !dir.join("tests.json").exists() {
            self.save_tests(&p.slug, &p.examples)?;
        }
        let code = self.code_path(&p.slug, p)?;
        if !code.exists() {
            write_atomic(&code, p.java_code.clone().unwrap_or_default().as_bytes())?;
        }
        self.update_state(&p.slug, |s| {
            if s.added_at == 0 {
                s.added_at = now();
            }
        })
    }

    pub fn load(&self, slug: &str) -> Result<Workspace, String> {
        let dir = self.dir(slug)?;
        let problem: Problem = read_json(&dir.join("problem.json"))?;
        let tests: Vec<TestCase> = read_json(&dir.join("tests.json")).unwrap_or_else(|_| problem.examples.clone());
        let code = std::fs::read_to_string(self.code_path(slug, &problem)?).unwrap_or_else(|_| problem.java_code.clone().unwrap_or_default());
        let state: ProblemState = read_json(&dir.join("state.json")).unwrap_or_default();
        Ok(Workspace { problem, tests, code, state })
    }

    pub fn save_code(&self, slug: &str, code: &str) -> Result<(), String> {
        let problem: Problem = read_json(&self.dir(slug)?.join("problem.json"))?;
        write_atomic(&self.code_path(slug, &problem)?, code.as_bytes())
    }

    pub fn save_tests(&self, slug: &str, tests: &[TestCase]) -> Result<(), String> {
        let dir = self.dir(slug)?;
        write_atomic(&dir.join("tests.json"), serde_json::to_string_pretty(tests).unwrap().as_bytes())
    }

    fn update_state(&self, slug: &str, f: impl FnOnce(&mut ProblemState)) -> Result<(), String> {
        let path = self.dir(slug)?.join("state.json");
        let mut s: ProblemState = read_json(&path).unwrap_or_default();
        f(&mut s);
        write_atomic(&path, serde_json::to_string_pretty(&s).unwrap().as_bytes())
    }

    pub fn touch(&self, slug: &str) -> Result<(), String> {
        self.update_state(slug, |s| s.opened_at = now())
    }

    pub fn set_verdict(&self, slug: &str, verdict: &str) -> Result<(), String> {
        self.update_state(slug, |s| {
            s.last_verdict = Some(verdict.to_string());
            if verdict == "accepted" {
                s.solved = true;
            }
        })
    }

    pub fn delete(&self, slug: &str) -> Result<(), String> {
        let dir = self.dir(slug)?;
        // Move to a trash folder instead of deleting, so nothing is lost by accident.
        let trash = self.root.join("trash");
        std::fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
        let dest = trash.join(format!("{slug}-{}", now()));
        std::fs::rename(&dir, &dest).map_err(|e| format!("cannot remove {slug}: {e}"))
    }

    pub fn list(&self) -> Vec<StoredSummary> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(self.root.join("problems")) else { return out };
        for e in rd.flatten() {
            let slug = e.file_name().to_string_lossy().to_string();
            let Ok(p) = read_json::<Problem>(&e.path().join("problem.json")) else {
                log::warn!("skipping {}: problem.json unreadable", e.path().display());
                continue;
            };
            let st: ProblemState = read_json(&e.path().join("state.json")).unwrap_or_default();
            // Problems saved before `added_at` existed: use when their folder was created.
            let added_at = if st.added_at > 0 {
                st.added_at
            } else {
                std::fs::metadata(e.path())
                    .and_then(|m| m.created().or_else(|_| m.modified()))
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0)
            };
            out.push(StoredSummary {
                slug,
                added_at,
                id: p.id,
                title: p.title,
                difficulty: p.difficulty,
                opened_at: st.opened_at,
                last_verdict: st.last_verdict,
                solved: st.solved,
            });
        }
        // Stable order: the order problems were added. Opening one never moves it.
        out.sort_by(|a, b| a.added_at.cmp(&b.added_at).then_with(|| a.slug.cmp(&b.slug)));
        out
    }

    pub fn is_empty(&self) -> bool {
        std::fs::read_dir(self.root.join("problems")).map(|mut r| r.next().is_none()).unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Meta;

    fn problem() -> Problem {
        Problem {
            id: "1".into(),
            title: "Two Sum".into(),
            slug: "two-sum".into(),
            difficulty: "Easy".into(),
            paid_only: false,
            content: String::new(),
            java_code: Some("class Solution {}".into()),
            meta: Some(Meta::Function { method: "twoSum".into(), params: vec![], return_type: "integer[]".into(), output_param: None }),
            examples: vec![TestCase { inputs: vec!["[1]".into()], expected: Some("[0]".into()) }],
            tags: vec![],
            hints: vec![],
            any_order: false,
            source: "sample".into(),
        }
    }

    #[test]
    fn roundtrip_keeps_code_on_refetch() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::new(d.path().to_path_buf()).unwrap();
        assert!(s.is_empty());
        s.save_problem(&problem()).unwrap();
        s.save_code("two-sum", "class Solution { int x; }").unwrap();
        s.save_problem(&problem()).unwrap();
        let w = s.load("two-sum").unwrap();
        assert_eq!(w.code, "class Solution { int x; }");
        assert_eq!(w.tests.len(), 1);
        s.set_verdict("two-sum", "accepted").unwrap();
        let l = s.list();
        assert_eq!(l.len(), 1);
        assert!(l[0].solved);
        s.delete("two-sum").unwrap();
        assert!(s.list().is_empty());
    }

    #[test]
    fn opening_a_problem_does_not_reorder_the_list() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::new(d.path().to_path_buf()).unwrap();
        for slug in ["first-one", "second-one", "third-one"] {
            let mut p = problem();
            p.slug = slug.into();
            s.save_problem(&p).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        let order = |s: &Store| s.list().into_iter().map(|x| x.slug).collect::<Vec<_>>();
        assert_eq!(order(&s), ["first-one", "second-one", "third-one"]);
        s.touch("third-one").unwrap();
        s.touch("first-one").unwrap();
        s.set_verdict("second-one", "accepted").unwrap();
        // Re-fetching keeps the original position too.
        let mut p = problem();
        p.slug = "first-one".into();
        s.save_problem(&p).unwrap();
        assert_eq!(order(&s), ["first-one", "second-one", "third-one"]);
        assert!(s.load("../x").is_err());
    }

    #[test]
    fn settings_are_clamped() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::new(d.path().to_path_buf()).unwrap();
        let saved = s.save_settings(&Settings { timeout_secs: 9999, memory_mb: 1, ..Settings::default() }).unwrap();
        assert_eq!(saved.timeout_secs, 60);
        assert_eq!(saved.memory_mb, 64);
        assert_eq!(s.settings().timeout_secs, 60);
    }
}
