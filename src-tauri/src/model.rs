//! Data shared between the backend modules and the UI (serialized as camelCase JSON).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Meta {
    #[serde(rename_all = "camelCase")]
    Function { method: String, params: Vec<Param>, return_type: String, output_param: Option<usize> },
    #[serde(rename_all = "camelCase")]
    Design { class_name: String, constructor_params: Vec<Param>, methods: Vec<String> },
}

impl Meta {
    /// Java file name the user's code compiles as.
    pub fn file_name(&self) -> String {
        match self {
            Meta::Function { .. } => "Solution.java".into(),
            Meta::Design { class_name, .. } => format!("{class_name}.java"),
        }
    }
    /// How many input lines make one test case.
    pub fn lines_per_test(&self) -> usize {
        match self {
            Meta::Function { params, .. } => params.len().max(1),
            Meta::Design { .. } => 2,
        }
    }
    pub fn input_labels(&self) -> Vec<String> {
        match self {
            Meta::Function { params, .. } => params.iter().map(|p| p.name.clone()).collect(),
            Meta::Design { .. } => vec!["calls".into(), "arguments".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    /// One raw LeetCode-format value per line (per parameter).
    pub inputs: Vec<String>,
    pub expected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub difficulty: String,
    pub paid_only: bool,
    pub content: String,
    pub java_code: Option<String>,
    /// LeetCode's Rust starter code. Missing on problems saved before Rust support.
    #[serde(default)]
    pub rust_code: Option<String>,
    pub meta: Option<Meta>,
    pub examples: Vec<TestCase>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub any_order: bool,
    /// "leetcode" or "sample".
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemSummary {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub difficulty: String,
    pub paid_only: bool,
}
