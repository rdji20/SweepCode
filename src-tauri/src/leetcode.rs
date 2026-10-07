//! LeetCode's public GraphQL API: fetch one problem, search the problem list,
//! and pull example outputs out of the description HTML.

use crate::model::{Meta, Param, Problem, ProblemSummary, TestCase};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::time::Duration;

const ENDPOINT: &str = "https://leetcode.com/graphql";
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) prob-warp/0.1";

const QUESTION_QUERY: &str = "query questionData($titleSlug: String!) { question(titleSlug: $titleSlug) { questionFrontendId title titleSlug content difficulty isPaidOnly exampleTestcases sampleTestCase metaData codeSnippets { langSlug code } topicTags { name } hints } }";
const LIST_QUERY: &str = "query problemsetQuestionList($categorySlug: String, $limit: Int, $skip: Int, $filters: QuestionListFilterInput) { questionList(categorySlug: $categorySlug, limit: $limit, skip: $skip, filters: $filters) { totalNum data { questionFrontendId title titleSlug difficulty isPaidOnly } } }";

/// What the user typed into the search bar.
#[derive(Debug, PartialEq)]
pub enum Lookup {
    Slug(String),
    Number(u32),
    Query(String),
}

static SLUG_IN_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"leetcode\.(?:com|cn)/problems/([a-z0-9-]+)").unwrap());
static SLUG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)+$").unwrap());

pub fn parse_lookup(input: &str) -> Lookup {
    let t = input.trim();
    if let Some(c) = SLUG_IN_URL.captures(t) {
        return Lookup::Slug(c[1].to_string());
    }
    let digits = t.trim_start_matches('#').trim_end_matches('.');
    if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = digits.parse() {
            return Lookup::Number(n);
        }
    }
    if SLUG.is_match(t) {
        return Lookup::Slug(t.to_string());
    }
    Lookup::Query(t.to_string())
}

pub fn valid_slug(s: &str) -> bool {
    !s.is_empty() && s.len() < 120 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

async fn graphql(query: &str, variables: Value, referer: &str) -> Result<Value, String> {
    let body = json!({ "query": query, "variables": variables });
    let resp = client()?
        .post(ENDPOINT)
        .header("Referer", referer)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("could not reach leetcode.com: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("reading leetcode.com reply failed: {e}"))?;
    if !status.is_success() {
        let snippet: String = text.chars().take(300).collect();
        return Err(format!("leetcode.com answered HTTP {status}: {snippet}"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("leetcode.com sent something that is not JSON ({e})"))?;
    if let Some(errs) = v.get("errors") {
        return Err(format!("leetcode.com GraphQL error: {errs}"));
    }
    Ok(v)
}

pub async fn search(query: &str, limit: u32) -> Result<Vec<ProblemSummary>, String> {
    let v = graphql(
        LIST_QUERY,
        json!({ "categorySlug": "", "limit": limit, "skip": 0, "filters": { "searchKeywords": query } }),
        "https://leetcode.com/problemset/",
    )
    .await?;
    let items = v["data"]["questionList"]["data"].as_array().cloned().unwrap_or_default();
    let mut out: Vec<ProblemSummary> = items
        .iter()
        .map(|q| ProblemSummary {
            id: q["questionFrontendId"].as_str().unwrap_or("").to_string(),
            title: q["title"].as_str().unwrap_or("").to_string(),
            slug: q["titleSlug"].as_str().unwrap_or("").to_string(),
            difficulty: q["difficulty"].as_str().unwrap_or("").to_string(),
            paid_only: q["isPaidOnly"].as_bool().unwrap_or(false),
        })
        .collect();
    // Exact number match first when the user typed a number.
    let q = query.trim();
    out.sort_by_key(|p| if p.id == q { 0 } else { 1 });
    Ok(out)
}

pub async fn fetch(slug: &str) -> Result<Problem, String> {
    if !valid_slug(slug) {
        return Err(format!("'{slug}' is not a valid problem slug"));
    }
    let v = graphql(
        QUESTION_QUERY,
        json!({ "titleSlug": slug }),
        &format!("https://leetcode.com/problems/{slug}/"),
    )
    .await?;
    let q = &v["data"]["question"];
    if q.is_null() {
        return Err(format!("LeetCode has no problem called '{slug}'"));
    }
    Ok(problem_from_json(q))
}

/// Resolves whatever the user typed to one problem.
pub async fn resolve(input: &str) -> Result<Problem, String> {
    match parse_lookup(input) {
        Lookup::Slug(s) => match fetch(&s).await {
            Ok(p) => Ok(p),
            Err(e) if e.starts_with("LeetCode has no problem") => first_search_hit(&s.replace('-', " ")).await,
            Err(e) => Err(e),
        },
        Lookup::Number(n) => {
            let hits = search(&n.to_string(), 20).await?;
            let hit = hits.into_iter().find(|h| h.id == n.to_string()).ok_or_else(|| format!("no problem number {n}"))?;
            fetch(&hit.slug).await
        }
        Lookup::Query(q) => first_search_hit(&q).await,
    }
}

async fn first_search_hit(q: &str) -> Result<Problem, String> {
    let hits = search(q, 5).await?;
    let hit = hits.into_iter().next().ok_or_else(|| format!("no problem matches '{q}'"))?;
    fetch(&hit.slug).await
}

pub fn problem_from_json(q: &Value) -> Problem {
    let s = |k: &str| q[k].as_str().unwrap_or("").to_string();
    let content = s("content");
    let meta = q["metaData"].as_str().and_then(|m| serde_json::from_str::<Value>(m).ok()).and_then(|m| parse_meta(&m));
    let java_code = q["codeSnippets"]
        .as_array()
        .and_then(|a| a.iter().find(|c| c["langSlug"] == "java"))
        .and_then(|c| c["code"].as_str())
        .map(|c| c.to_string());
    let examples = build_examples(&s("exampleTestcases"), meta.as_ref(), &content);
    let text = html_to_text(&content).to_lowercase();
    Problem {
        id: s("questionFrontendId"),
        title: s("title"),
        slug: s("titleSlug"),
        difficulty: s("difficulty"),
        paid_only: q["isPaidOnly"].as_bool().unwrap_or(false),
        content,
        java_code,
        meta,
        examples,
        tags: q["topicTags"].as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect()).unwrap_or_default(),
        hints: q["hints"].as_array().map(|a| a.iter().filter_map(|h| h.as_str().map(String::from)).collect()).unwrap_or_default(),
        any_order: text.contains("any order") || text.contains("order does not matter") || text.contains("order of the output does not matter"),
        source: "leetcode".into(),
    }
}

pub fn parse_meta(m: &Value) -> Option<Meta> {
    let params = |v: &Value| -> Vec<Param> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .map(|p| Param {
                        name: p["name"].as_str().unwrap_or("").to_string(),
                        ty: p["type"].as_str().unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    if m["systemdesign"].as_bool() == Some(true) || m.get("classname").is_some() {
        return Some(Meta::Design {
            class_name: m["classname"].as_str()?.to_string(),
            constructor_params: params(&m["constructor"]["params"]),
            methods: m["methods"].as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect()).unwrap_or_default(),
        });
    }
    Some(Meta::Function {
        method: m["name"].as_str()?.to_string(),
        params: params(&m["params"]),
        return_type: m["return"]["type"].as_str().unwrap_or("").to_string(),
        output_param: m["output"]["paramindex"].as_u64().map(|x| x as usize),
    })
}

fn build_examples(raw: &str, meta: Option<&Meta>, content: &str) -> Vec<TestCase> {
    let per = match meta {
        Some(Meta::Function { params, .. }) => params.len().max(1),
        Some(Meta::Design { .. }) => 2,
        None => return Vec::new(),
    };
    let mut lines: Vec<&str> = raw.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines.pop();
    }
    let outputs = parse_outputs(content);
    lines
        .chunks(per)
        .filter(|c| c.len() == per)
        .enumerate()
        .map(|(i, c)| TestCase {
            inputs: c.iter().map(|s| s.to_string()).collect(),
            expected: outputs.get(i).cloned(),
        })
        .collect()
}

static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]*>").unwrap());
static BLOCK_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>|</p>|</pre>|</div>|</li>").unwrap());
static NUM_ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#(x[0-9a-fA-F]+|\d+);").unwrap());
static OUTPUT_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*Output:?\s*(.*)$").unwrap());

pub fn html_to_text(html: &str) -> String {
    let t = BLOCK_END.replace_all(html, "\n");
    let t = TAG.replace_all(&t, "");
    let t = NUM_ENTITY.replace_all(&t, |c: &regex::Captures| {
        let s = &c[1];
        let n = if let Some(hex) = s.strip_prefix('x') { u32::from_str_radix(hex, 16).ok() } else { s.parse().ok() };
        n.and_then(char::from_u32).map(|ch| ch.to_string()).unwrap_or_default()
    });
    t.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Expected outputs of the examples, in order.
pub fn parse_outputs(html: &str) -> Vec<String> {
    let text = html_to_text(html);
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(c) = OUTPUT_LINE.captures(lines[i]) {
            let mut v = c[1].trim().to_string();
            if v.is_empty() {
                // Design problems put the value on the next line.
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }
                if j < lines.len() {
                    v = lines[j].trim().to_string();
                    i = j;
                }
            }
            if !v.is_empty() {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_parsing() {
        assert_eq!(parse_lookup("https://leetcode.com/problems/two-sum/description/"), Lookup::Slug("two-sum".into()));
        assert_eq!(parse_lookup("leetcode.com/problems/lru-cache"), Lookup::Slug("lru-cache".into()));
        assert_eq!(parse_lookup("146"), Lookup::Number(146));
        assert_eq!(parse_lookup("#1"), Lookup::Number(1));
        assert_eq!(parse_lookup("two-sum"), Lookup::Slug("two-sum".into()));
        assert_eq!(parse_lookup("Two Sum"), Lookup::Query("Two Sum".into()));
        assert!(!valid_slug("../etc"));
    }

    #[test]
    fn outputs_old_pre_format() {
        let html = "<pre>\n<strong>Input:</strong> nums = [2,7,11,15], target = 9\n<strong>Output:</strong> [0,1]\n<strong>Explanation:</strong> x\n</pre><pre><strong>Input:</strong> s = &quot;abc&quot;\n<strong>Output:</strong> &quot;cba&quot;\n</pre>";
        assert_eq!(parse_outputs(html), vec!["[0,1]".to_string(), "\"cba\"".to_string()]);
    }

    #[test]
    fn outputs_new_example_block_format() {
        let html = r#"<div class="example-block"><p><strong>Input:</strong> <span class="example-io">nums = [1,2]</span></p><p><strong>Output:</strong> <span class="example-io">3</span></p><p><strong>Explanation:</strong></p></div>"#;
        assert_eq!(parse_outputs(html), vec!["3".to_string()]);
    }

    #[test]
    fn outputs_design_format() {
        let html = "<pre><strong>Input</strong>\n[&quot;LRUCache&quot;, &quot;put&quot;]\n[[2], [1, 1]]\n<strong>Output</strong>\n[null, null]\n\n<strong>Explanation</strong>\nfoo\n</pre>";
        assert_eq!(parse_outputs(html), vec!["[null, null]".to_string()]);
    }

    #[test]
    fn meta_function_and_design() {
        let f: Value = serde_json::from_str(r#"{"name":"rotate","params":[{"name":"nums","type":"integer[]"},{"name":"k","type":"integer"}],"return":{"type":"void"},"output":{"paramindex":0}}"#).unwrap();
        match parse_meta(&f).unwrap() {
            Meta::Function { method, params, output_param, .. } => {
                assert_eq!(method, "rotate");
                assert_eq!(params.len(), 2);
                assert_eq!(output_param, Some(0));
            }
            _ => panic!(),
        }
        let d: Value = serde_json::from_str(r#"{"classname":"LRUCache","constructor":{"params":[{"type":"integer","name":"capacity"}]},"methods":[{"name":"get"}],"systemdesign":true}"#).unwrap();
        assert!(matches!(parse_meta(&d).unwrap(), Meta::Design { .. }));
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_fetch_and_search() {
        let p = fetch("two-sum").await.unwrap();
        assert_eq!(p.examples.len(), 3);
        assert_eq!(p.examples[0].expected.as_deref(), Some("[0,1]"));
        let lru = resolve("146").await.unwrap();
        assert_eq!(lru.slug, "lru-cache");
        assert!(matches!(lru.meta, Some(Meta::Design { .. })));
        assert_eq!(lru.examples[0].inputs.len(), 2);
        assert!(lru.examples[0].expected.as_ref().unwrap().starts_with("[null"));
        let hits = search("palindrome", 5).await.unwrap();
        assert!(!hits.is_empty());
        // Premium problem: LeetCode hides content and code; we must not crash.
        let paid = fetch("meeting-rooms").await.unwrap();
        assert!(paid.paid_only);
        assert!(paid.content.is_empty());
        assert!(paid.java_code.is_none());
        // SQL problem: no Java snippet.
        let sql = fetch("combine-two-tables").await.unwrap();
        assert!(sql.java_code.is_none());
    }
}
