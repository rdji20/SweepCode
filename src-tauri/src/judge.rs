//! Compares a program's output with the expected output, LeetCode style.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Match {
    /// Same text once whitespace outside strings is ignored.
    Exact,
    /// Same values, e.g. `2` vs `2.00000` or decimals within 1e-5.
    Equivalent,
    /// Same elements in a different order (problem says "any order").
    AnyOrder,
    No,
}

impl Match {
    pub fn passed(&self) -> bool {
        !matches!(self, Match::No)
    }
    pub fn note(&self) -> Option<String> {
        match self {
            Match::Equivalent => Some("matched within 1e-5".into()),
            Match::AnyOrder => Some("same elements, different order (allowed: any order)".into()),
            _ => None,
        }
    }
}

/// Removes whitespace that is not inside a double-quoted string.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_str = false;
    let mut escaped = false;
    for c in s.trim().chars() {
        if in_str {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if !c.is_whitespace() {
            out.push(c);
        }
    }
    out
}

fn num_eq(a: f64, b: f64) -> bool {
    let diff = (a - b).abs();
    diff <= 1e-5 || diff <= 1e-5 * a.abs().max(b.abs())
}

fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(p), Some(q)) => num_eq(p, q),
            _ => x == y,
        },
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| value_eq(p, q)),
        _ => a == b,
    }
}

fn sort_key(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

fn sorted_top(v: &Value) -> Value {
    match v {
        Value::Array(items) => {
            let mut items = items.clone();
            items.sort_by_key(sort_key);
            Value::Array(items)
        }
        other => other.clone(),
    }
}

fn sorted_deep(v: &Value) -> Value {
    match v {
        Value::Array(items) => {
            let mut items: Vec<Value> = items.iter().map(sorted_deep).collect();
            items.sort_by_key(sort_key);
            Value::Array(items)
        }
        other => other.clone(),
    }
}

pub fn compare(actual: &str, expected: &str, any_order: bool) -> Match {
    let a = normalize(actual);
    let e = normalize(expected);
    if a == e {
        return Match::Exact;
    }
    let (Ok(av), Ok(ev)) = (serde_json::from_str::<Value>(&a), serde_json::from_str::<Value>(&e)) else {
        return Match::No;
    };
    if value_eq(&av, &ev) {
        return Match::Equivalent;
    }
    if any_order && (value_eq(&sorted_top(&av), &sorted_top(&ev)) || value_eq(&sorted_deep(&av), &sorted_deep(&ev))) {
        return Match::AnyOrder;
    }
    Match::No
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_outside_strings_is_ignored() {
        assert_eq!(compare("[0,1]", "[0, 1]", false), Match::Exact);
        assert_eq!(compare("[null,null,1]", "[null, null, 1]", false), Match::Exact);
        assert_eq!(compare("\"a b\"", "\"a b\"", false), Match::Exact);
        assert_eq!(compare("\"ab\"", "\"a b\"", false), Match::No);
    }

    #[test]
    fn numbers_within_tolerance() {
        assert_eq!(compare("2.00000", "2.00000", false), Match::Exact);
        assert_eq!(compare("2.50000", "2.5", false), Match::Equivalent);
        assert_eq!(compare("2", "2.00000", false), Match::Equivalent);
        assert_eq!(compare("2.00001", "2.00000", false), Match::Equivalent);
        assert_eq!(compare("2.1", "2.0", false), Match::No);
    }

    #[test]
    fn any_order() {
        assert_eq!(compare("[1,0]", "[0,1]", false), Match::No);
        assert_eq!(compare("[1,0]", "[0,1]", true), Match::AnyOrder);
        assert_eq!(compare("[[\"tan\",\"nat\"],[\"bat\"]]", "[[\"bat\"],[\"nat\",\"tan\"]]", true), Match::AnyOrder);
        assert_eq!(compare("[1,1,2]", "[1,2,2]", true), Match::No);
    }

    #[test]
    fn non_json_text() {
        assert_eq!(compare("hello", "hello", false), Match::Exact);
        assert_eq!(compare("2, nums = [1,2,_]", "2, nums = [1,2,_]", false), Match::Exact);
        assert_eq!(compare("true", "false", false), Match::No);
    }
}
