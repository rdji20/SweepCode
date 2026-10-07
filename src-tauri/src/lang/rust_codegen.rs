//! Writes the `main` that calls the user's Rust solution.
//!
//! Rust has no runtime reflection, so instead of one fixed driver (like Java),
//! we read the method signature from the user's own code (exact Rust types,
//! including `&mut Vec<i32>`) and generate code that parses each input into that
//! type, calls the method, and prints the result. The runtime (`pw`) does the
//! parsing and printing through the `FromJson` / `ToJson` traits.

use crate::model::Meta;

#[derive(Debug, Clone, PartialEq)]
pub enum Pass {
    Value,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    /// The owned type to parse into, e.g. `Vec<i32>` for `&mut Vec<i32>` or `&[i32]`.
    pub owned: String,
    pub pass: Pass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sig {
    pub params: Vec<Param>,
    /// Return type text, "()" when there is none.
    pub ret: String,
}

/// Replaces comments with spaces (same length, newlines kept) so searching
/// the code never matches inside a comment. String literals are kept as is.
pub fn strip_comments(code: &str) -> String {
    let b = code.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        if in_str {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i] == b'"' {
                in_str = false;
            }
            i += 1;
        } else if b[i] == b'"' {
            in_str = true;
            i += 1;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if b[i] != b'\n' && b[i] < 0x80 {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| code.to_string())
}

/// LeetCode's camelCase method names become snake_case in Rust: twoSum -> two_sum.
pub fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
            let next_lower = chars.get(i + 1).map(|n| n.is_lowercase()).unwrap_or(false);
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            if i > 0 && (prev_lower || (prev_upper && next_lower)) {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte range of the `{ ... }` body of `impl <ty>`, if present.
pub fn impl_block(code: &str, ty: &str) -> Option<(usize, usize)> {
    let mut from = 0;
    while let Some(pos) = code[from..].find("impl") {
        let start = from + pos;
        from = start + 4;
        if start > 0 && is_ident(code[..start].chars().last().unwrap()) {
            continue;
        }
        let rest = code[start + 4..].trim_start();
        let Some(after) = rest.strip_prefix(ty) else { continue };
        if after.chars().next().map(is_ident).unwrap_or(false) {
            continue;
        }
        let open = start + 4 + (code[start + 4..].len() - rest.len()) + ty.len() + after.find('{')?;
        let mut depth = 0;
        for (j, c) in code[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((open + 1, open + j));
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    None
}

/// Finds `fn <name>(...) -> Ret` inside `code` and parses it.
pub fn find_fn(code: &str, name: &str) -> Option<Sig> {
    let mut from = 0;
    loop {
        let pos = code[from..].find("fn ")? + from;
        from = pos + 3;
        if pos > 0 && is_ident(code[..pos].chars().last().unwrap()) {
            continue;
        }
        let rest = code[pos + 3..].trim_start();
        let Some(after) = rest.strip_prefix(name) else { continue };
        let after_trim = after.trim_start();
        if !after_trim.starts_with('(') {
            continue;
        }
        // Parameter list up to the matching ')'.
        let mut depth = 0i32;
        let mut end = None;
        for (j, c) in after_trim.char_indices() {
            match c {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => {
                    // `->` inside generics never appears in a parameter list
                    depth -= 1;
                    if depth == 0 && c == ')' {
                        end = Some(j);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end?;
        let inside = &after_trim[1..end];
        let tail = after_trim[end + 1..].trim_start();
        let ret = if let Some(r) = tail.strip_prefix("->") {
            let r = r.trim_start();
            let stop = r.find(['{', ';']).unwrap_or(r.len());
            let r = r[..stop].trim();
            let r = r.strip_suffix("where").unwrap_or(r).trim();
            r.to_string()
        } else {
            "()".to_string()
        };
        let mut params = Vec::new();
        for part in split_top(inside) {
            let part = part.trim();
            if part.is_empty() || part.contains("self") {
                continue;
            }
            let (name, ty) = part.split_once(':')?;
            let name = name.trim().trim_start_matches("mut ").trim().to_string();
            params.push(param(name, ty.trim()));
        }
        return Some(Sig { params, ret });
    }
}

fn split_top(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '<' | '(' | '[' => {
                depth += 1;
                cur.push(c)
            }
            '>' | ')' | ']' => {
                depth -= 1;
                cur.push(c)
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

fn owned_of(ty: &str) -> String {
    let t = ty.trim();
    if t == "str" {
        return "String".into();
    }
    if let Some(inner) = t.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        if !inner.contains(';') {
            return format!("Vec<{}>", owned_of(inner));
        }
    }
    t.to_string()
}

fn param(name: String, ty: &str) -> Param {
    let ty = ty.trim();
    if let Some(rest) = ty.strip_prefix("&mut ") {
        Param { name, owned: owned_of(rest), pass: Pass::RefMut }
    } else if let Some(rest) = ty.strip_prefix('&') {
        Param { name, owned: owned_of(rest.trim_start_matches("'_ ").trim()), pass: Pass::Ref }
    } else {
        Param { name, owned: owned_of(ty), pass: Pass::Value }
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Declares `aN` from a raw input line and returns the expression to pass it.
fn bind(i: usize, p: &Param, label: &str, source: &str) -> (String, String) {
    let decl = format!(
        "        let {}a{i}: {} = {source}?;\n",
        if p.pass == Pass::RefMut { "mut " } else { "" },
        p.owned
    );
    let _ = label;
    let pass = match p.pass {
        Pass::Value => format!("a{i}"),
        Pass::Ref => format!("&a{i}"),
        Pass::RefMut => format!("&mut a{i}"),
    };
    (decl, pass)
}

/// Generates the body of `main`. Errors are explained in plain words.
pub fn generate(meta: &Meta, code: &str) -> Result<String, String> {
    let clean = strip_comments(code);
    match meta {
        Meta::Function { method, params, output_param, .. } => {
            let fname = snake(method);
            let scope = impl_block(&clean, "Solution").map(|(a, b)| &clean[a..b]).unwrap_or(&clean);
            let sig = find_fn(scope, &fname)
                .or_else(|| find_fn(scope, method))
                .ok_or_else(|| format!("fn {fname}(...) was not found in impl Solution. Keep the method name LeetCode gave you."))?;
            if sig.params.len() != params.len() {
                return Err(format!(
                    "fn {fname} takes {} parameter(s) but each test gives {}.",
                    sig.params.len(),
                    params.len()
                ));
            }
            let call_name = if find_fn(scope, &fname).is_some() { fname.clone() } else { method.clone() };
            let per = params.len().max(1);
            let mut body = String::new();
            let mut args = Vec::new();
            for (i, p) in sig.params.iter().enumerate() {
                let label = params.get(i).map(|x| x.name.as_str()).unwrap_or(&p.name);
                let (decl, pass) = bind(i, p, label, &format!("pw::arg(&raw[{i}], \"{}\")", esc(label)));
                body.push_str(&decl);
                args.push(pass);
            }
            let out = if sig.ret == "()" {
                match output_param {
                    Some(k) if *k < sig.params.len() => format!("pw::ToJson::to_json(&a{k})"),
                    _ => "\"null\".to_string()".into(),
                }
            } else {
                "pw::ToJson::to_json(&r)".into()
            };
            Ok(format!(
                r#"    let job = pw::start();
    let per = {per};
    if job.inputs.len() % per != 0 {{
        pw::fatal(&format!("each test needs {{}} input lines but there are {{}} lines in total", per, job.inputs.len()));
        pw::end();
        return;
    }}
    for (i, chunk) in job.inputs.chunks(per).enumerate() {{
        let raw: Vec<String> = chunk.to_vec();
        pw::run_case(i, move || {{
{body}        let t = std::time::Instant::now();
        let r = Solution::{call_name}({args});
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let _ = &r;
        Ok(({out}, ms))
        }});
    }}
    pw::end();
"#,
                args = args.join(", ")
            ))
        }
        Meta::Design { class_name, methods, constructor_params } => {
            let scope = impl_block(&clean, class_name)
                .map(|(a, b)| &clean[a..b])
                .ok_or_else(|| format!("impl {class_name} was not found. Keep the struct name LeetCode gave you."))?;
            let ctor = find_fn(scope, "new").ok_or_else(|| format!("fn new(...) was not found in impl {class_name}."))?;
            let mut arms = String::new();
            // constructor
            let mut decls = String::new();
            let mut args = Vec::new();
            for (i, p) in ctor.params.iter().enumerate() {
                let label = constructor_params.get(i).map(|x| x.name.as_str()).unwrap_or(&p.name);
                let (decl, pass) = bind(i, p, label, &format!("pw::call_arg(&args, {i}, \"{}\", \"{}\")", esc(label), esc(class_name)));
                decls.push_str(&format!("    {decl}"));
                args.push(pass);
            }
            arms.push_str(&format!(
                "                \"{cn}\" => {{\n{decls}                    let t = std::time::Instant::now();\n                    obj = Some({cn}::new({a}));\n                    ms += t.elapsed().as_secs_f64() * 1000.0;\n                    out.push(\"null\".to_string());\n                }}\n",
                cn = class_name,
                a = args.join(", ")
            ));
            for m in methods {
                let f = snake(m);
                let sig = find_fn(scope, &f)
                    .or_else(|| find_fn(scope, m))
                    .ok_or_else(|| format!("fn {f}(...) was not found in impl {class_name}."))?;
                let call = if find_fn(scope, &f).is_some() { f.clone() } else { m.clone() };
                let mut decls = String::new();
                let mut args = Vec::new();
                for (i, p) in sig.params.iter().enumerate() {
                    let (decl, pass) = bind(i, p, &p.name, &format!("pw::call_arg(&args, {i}, \"{}\", \"{}\")", esc(&p.name), esc(m)));
                    decls.push_str(&format!("    {decl}"));
                    args.push(pass);
                }
                let push = if sig.ret == "()" { "out.push(\"null\".to_string());".to_string() } else { "out.push(pw::ToJson::to_json(&r));".to_string() };
                arms.push_str(&format!(
                    "                \"{m}\" => {{\n                    let o = obj.as_mut().ok_or_else(|| \"the first call must create {cn}\".to_string())?;\n{decls}                    let t = std::time::Instant::now();\n                    let r = o.{call}({a});\n                    ms += t.elapsed().as_secs_f64() * 1000.0;\n                    let _ = &r;\n                    {push}\n                }}\n",
                    m = esc(m),
                    cn = class_name,
                    a = args.join(", ")
                ));
            }
            Ok(format!(
                r#"    let job = pw::start();
    if job.inputs.len() % 2 != 0 {{
        pw::fatal("design tests need 2 lines each (method names, then arguments)");
        pw::end();
        return;
    }}
    for (i, chunk) in job.inputs.chunks(2).enumerate() {{
        let raw: Vec<String> = chunk.to_vec();
        pw::run_case(i, move || {{
        let names: Vec<String> = pw::arg(&raw[0], "calls")?;
        let calls: Vec<pw::Json> = pw::arg(&raw[1], "arguments")?;
        if names.len() != calls.len() {{
            return Err(format!("{{}} calls but {{}} argument lists", names.len(), calls.len()));
        }}
        let mut obj: Option<{cn}> = None;
        let mut out: Vec<String> = Vec::new();
        let mut ms = 0.0f64;
        for (k, name) in names.iter().enumerate() {{
            let args: Vec<pw::Json> = match &calls[k] {{
                pw::Json::Arr(a) => a.clone(),
                _ => return Err(format!("the arguments of call {{}} must be an array", k + 1)),
            }};
            match name.as_str() {{
{arms}                other => return Err(format!("{cn} has no method {{other}}")),
            }}
        }}
        Ok((format!("[{{}}]", out.join(",")), ms))
        }});
    }}
    pw::end();
"#,
                cn = class_name
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_like_leetcode() {
        assert_eq!(snake("twoSum"), "two_sum");
        assert_eq!(snake("findMedianSortedArrays"), "find_median_sorted_arrays");
        assert_eq!(snake("getRandom"), "get_random");
        assert_eq!(snake("rotate"), "rotate");
        assert_eq!(snake("maxProfitII"), "max_profit_ii");
        assert_eq!(snake("kthSmallest"), "kth_smallest");
    }

    #[test]
    fn comments_are_blanked_but_strings_kept() {
        let c = "// fn two_sum(x: i32)\nlet s = \"// not a comment\"; /* fn a() */ fn b() {}";
        let s = strip_comments(c);
        assert_eq!(s.len(), c.len());
        assert!(!s.contains("two_sum"));
        assert!(s.contains("\"// not a comment\""));
        assert!(!s.contains("fn a"));
        assert!(s.contains("fn b"));
    }

    #[test]
    fn parses_leetcode_signatures() {
        let code = "impl Solution {\n    pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {\n    }\n    pub fn rotate(nums: &mut Vec<i32>, k: i32) {\n    }\n    pub fn f(mut s: String, grid: &[Vec<char>], t: &str) -> Option<Box<ListNode>> {}\n}";
        let s = find_fn(code, "two_sum").unwrap();
        assert_eq!(s.ret, "Vec<i32>");
        assert_eq!(s.params[0], Param { name: "nums".into(), owned: "Vec<i32>".into(), pass: Pass::Value });
        let r = find_fn(code, "rotate").unwrap();
        assert_eq!(r.ret, "()");
        assert_eq!(r.params[0].pass, Pass::RefMut);
        assert_eq!(r.params[0].owned, "Vec<i32>");
        let f = find_fn(code, "f").unwrap();
        assert_eq!(f.params[0].name, "s");
        assert_eq!(f.params[1].owned, "Vec<Vec<char>>");
        assert_eq!(f.params[1].pass, Pass::Ref);
        assert_eq!(f.params[2].owned, "String");
        assert_eq!(f.ret, "Option<Box<ListNode>>");
        assert!(find_fn(code, "two").is_none());
    }

    #[test]
    fn finds_impl_blocks() {
        let code = "struct LRUCache { m: i32 }\nimpl LRUCache {\n fn new(capacity: i32) -> Self { LRUCache { m: 0 } }\n fn get(&self, key: i32) -> i32 { 0 }\n}\nimpl LRUCacheX {}";
        let (a, b) = impl_block(code, "LRUCache").unwrap();
        assert!(code[a..b].contains("fn get"));
        assert!(impl_block(code, "Solution").is_none());
    }
}
