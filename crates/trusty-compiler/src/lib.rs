pub mod codegen;
pub mod parser;
pub mod stdlib;
pub mod transpiler;

use anyhow::{bail, Result};

pub use transpiler::TranspileOutput;

/// Transpile TRUST source to Rust source code.
pub fn compile(source: &str) -> Result<String> {
    Ok(compile_full(source)?.rust_code)
}

/// Transpile TRUST source and return Rust code + required external crates.
pub fn compile_full(source: &str) -> Result<TranspileOutput> {
    reject_unsupported_while(source)?;
    warn_on_deprecated_number_alias(source);
    let preprocessed = preprocess(source);
    let ast = parser::parse_typescript(&preprocessed)?;
    transpiler::transpile_to_rust(&ast)
}

fn warn_on_deprecated_number_alias(source: &str) {
    if !contains_identifier_in_code(source, "number") {
        return;
    }
    eprintln!("⚠️  Deprecated type alias `number` detected. Prefer `int` (or `int32`) / `float`.");
}

fn reject_unsupported_while(source: &str) -> Result<()> {
    if contains_identifier_in_code(source, "while") {
        bail!("`while` is not supported in TRUST. Use `loop (condition) {{ ... }}` instead.");
    }
    Ok(())
}

fn contains_identifier_in_code(source: &str, needle: &str) -> bool {
    let mut ident = String::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;

    while i < chars.len() {
        let ch = chars[i];
        let next = if i + 1 < chars.len() {
            Some(chars[i + 1])
        } else {
            None
        };

        if in_single {
            if ch == '\\' && next.is_some() {
                i += 2;
                continue;
            }
            if ch == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if ch == '\\' && next.is_some() {
                i += 2;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if in_template {
            if ch == '\\' && next.is_some() {
                i += 2;
                continue;
            }
            if ch == '`' {
                in_template = false;
            }
            i += 1;
            continue;
        }

        if ch == '/' && next == Some('/') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if ch == '\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if ch == '"' {
            in_double = true;
            i += 1;
            continue;
        }
        if ch == '`' {
            in_template = true;
            i += 1;
            continue;
        }

        if ch.is_ascii_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else {
            if ident == needle {
                return true;
            }
            ident.clear();
        }
        i += 1;
    }

    ident == needle
}

/// Rewrite TRUST-specific keywords to valid TypeScript before SWC parsing.
fn preprocess(source: &str) -> String {
    rewrite_word_boolean_ops(&rewrite_val_declarations(&rewrite_implements_blocks(&rewrite_match_blocks(source))))
        .replace("struct ", "interface ")
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            // `wait expr;` → `(expr).join().unwrap();`
            if let Some(rest) = trimmed.strip_prefix("wait ") {
                let indent = &line[..line.len() - trimmed.len()];
                let rest = rest.trim_end_matches(';').trim();
                format!("{}({}).join().unwrap();", indent, rest)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_match_blocks(source: &str) -> String {
    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0usize;
    let mut match_id = 0usize;

    while i < chars.len() {
        let can_start = i + 5 <= chars.len()
            && chars[i..i + 5].iter().collect::<String>() == "match"
            && (i == 0 || !is_ident_char(chars[i - 1]))
            && (i + 5 >= chars.len() || !is_ident_char(chars[i + 5]));

        if !can_start {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let mut j = i + 5;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j >= chars.len() || chars[j] != '(' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let Some(subject_end) = find_matching(&chars, j, '(', ')') else {
            out.push(chars[i]);
            i += 1;
            continue;
        };
        let subject = chars[j + 1..subject_end].iter().collect::<String>().trim().to_string();

        let mut k = subject_end + 1;
        while k < chars.len() && chars[k].is_whitespace() {
            k += 1;
        }
        if k >= chars.len() || chars[k] != '{' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let Some(body_end) = find_matching(&chars, k, '{', '}') else {
            out.push(chars[i]);
            i += 1;
            continue;
        };

        let body = chars[k + 1..body_end].iter().collect::<String>();
        if let Some(rewritten) = build_match_expr(&subject, &body, match_id) {
            out.push_str(&rewritten);
            match_id += 1;
            i = body_end + 1;
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

fn find_matching(chars: &[char], open_pos: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = open_pos;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    while i < chars.len() {
        let c = chars[i];
        let next = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

        if in_single {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if in_template {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '`' {
                in_template = false;
            }
            i += 1;
            continue;
        }

        if c == '\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if c == '"' {
            in_double = true;
            i += 1;
            continue;
        }
        if c == '`' {
            in_template = true;
            i += 1;
            continue;
        }

        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn split_top_level_csv(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut par = 0i32;
    let mut brk = 0i32;
    let mut brc = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;

    while i < chars.len() {
        let c = chars[i];
        let next = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

        if in_single {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if in_template {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '`' {
                in_template = false;
            }
            i += 1;
            continue;
        }

        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '`' => in_template = true,
            '(' => par += 1,
            ')' => par -= 1,
            '[' => brk += 1,
            ']' => brk -= 1,
            '{' => brc += 1,
            '}' => brc -= 1,
            ',' if par == 0 && brk == 0 && brc == 0 => {
                let part = chars[start..i].iter().collect::<String>().trim().to_string();
                if !part.is_empty() {
                    parts.push(part);
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }

    let tail = chars[start..].iter().collect::<String>().trim().to_string();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

fn find_top_level_arrow(input: &str) -> Option<usize> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    let mut par = 0i32;
    let mut brk = 0i32;
    let mut brc = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    while i < chars.len() {
        let c = chars[i];
        let next = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

        if in_single {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if in_template {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '`' {
                in_template = false;
            }
            i += 1;
            continue;
        }

        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '`' => in_template = true,
            '(' => par += 1,
            ')' => par -= 1,
            '[' => brk += 1,
            ']' => brk -= 1,
            '{' => brc += 1,
            '}' => brc -= 1,
            '=' if next == Some('>') && par == 0 && brk == 0 && brc == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn build_match_expr(subject: &str, body: &str, match_id: usize) -> Option<String> {
    let arms = split_top_level_csv(body);
    if arms.is_empty() {
        return None;
    }

    let mut conditions: Vec<(String, String)> = Vec::new();
    let mut default_expr: Option<String> = None;

    for arm in arms {
        let arrow = find_top_level_arrow(&arm)?;
        let pattern = arm[..arrow].trim();
        let expr = arm[arrow + 2..].trim();
        if pattern.is_empty() || expr.is_empty() {
            return None;
        }
        if pattern == "default" {
            default_expr = Some(expr.to_string());
            continue;
        }
        if pattern.starts_with('[') && pattern.ends_with(']') {
            let list_inner = pattern[1..pattern.len() - 1].trim();
            let cond = format!("[{}].contains(&__trust_match_{})", list_inner, match_id);
            conditions.push((cond, expr.to_string()));
        } else {
            let cond = format!("__trust_match_{} == ({})", match_id, pattern);
            conditions.push((cond, expr.to_string()));
        }
    }

    let mut out = format!("({{ let __trust_match_{} = {}; ", match_id, subject);
    if conditions.is_empty() {
        let d = default_expr?;
        out.push_str(&format!("{} }})", d));
        return Some(out);
    }

    for (idx, (cond, expr)) in conditions.iter().enumerate() {
        if idx == 0 {
            out.push_str(&format!("if {} {{ {} }} ", cond, expr));
        } else {
            out.push_str(&format!("else if {} {{ {} }} ", cond, expr));
        }
    }

    if let Some(d) = default_expr {
        out.push_str(&format!("else {{ {} }} ", d));
    } else {
        out.push_str("else { panic!(\"non-exhaustive match\") } ");
    }
    out.push_str("})");
    Some(out)
}

fn rewrite_word_boolean_ops(source: &str) -> String {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Normal,
        LineComment,
        BlockComment,
        SingleString,
        DoubleString,
        TemplateString,
    }

    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let mut mode = Mode::Normal;

    while i < chars.len() {
        let c = chars[i];
        let next = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

        match mode {
            Mode::Normal => {
                if c == '/' && next == Some('/') {
                    out.push(c);
                    out.push('/');
                    i += 2;
                    mode = Mode::LineComment;
                    continue;
                }
                if c == '/' && next == Some('*') {
                    out.push(c);
                    out.push('*');
                    i += 2;
                    mode = Mode::BlockComment;
                    continue;
                }
                if c == '\'' {
                    out.push(c);
                    i += 1;
                    mode = Mode::SingleString;
                    continue;
                }
                if c == '"' {
                    out.push(c);
                    i += 1;
                    mode = Mode::DoubleString;
                    continue;
                }
                if c == '`' {
                    out.push(c);
                    i += 1;
                    mode = Mode::TemplateString;
                    continue;
                }

                // Replace standalone `and`/`or` identifiers
                if c == 'a' && i + 2 < chars.len() && chars[i + 1] == 'n' && chars[i + 2] == 'd' {
                    let prev_ok = i == 0 || !is_ident_char(chars[i - 1]);
                    let next_ok = i + 3 >= chars.len() || !is_ident_char(chars[i + 3]);
                    if prev_ok && next_ok {
                        out.push_str("&&");
                        i += 3;
                        continue;
                    }
                }
                if c == 'o' && i + 1 < chars.len() && chars[i + 1] == 'r' {
                    let prev_ok = i == 0 || !is_ident_char(chars[i - 1]);
                    let next_ok = i + 2 >= chars.len() || !is_ident_char(chars[i + 2]);
                    if prev_ok && next_ok {
                        out.push_str("||");
                        i += 2;
                        continue;
                    }
                }
                if c == 'l'
                    && i + 3 < chars.len()
                    && chars[i + 1] == 'o'
                    && chars[i + 2] == 'o'
                    && chars[i + 3] == 'p'
                {
                    let prev_ok = i == 0 || !is_ident_char(chars[i - 1]);
                    let mut k = i + 4;
                    while k < chars.len() && chars[k].is_whitespace() {
                        k += 1;
                    }
                    let next_ok = (i + 4 >= chars.len() || !is_ident_char(chars[i + 4]))
                        && k < chars.len()
                        && chars[k] == '(';
                    if prev_ok && next_ok {
                        out.push_str("while");
                        i += 4;
                        continue;
                    }
                }

                out.push(c);
                i += 1;
            }
            Mode::LineComment => {
                out.push(c);
                i += 1;
                if c == '\n' {
                    mode = Mode::Normal;
                }
            }
            Mode::BlockComment => {
                out.push(c);
                i += 1;
                if c == '*' && next == Some('/') {
                    out.push('/');
                    i += 1;
                    mode = Mode::Normal;
                }
            }
            Mode::SingleString => {
                out.push(c);
                i += 1;
                if c == '\\' {
                    if let Some(n) = next {
                        out.push(n);
                        i += 1;
                    }
                } else if c == '\'' {
                    mode = Mode::Normal;
                }
            }
            Mode::DoubleString => {
                out.push(c);
                i += 1;
                if c == '\\' {
                    if let Some(n) = next {
                        out.push(n);
                        i += 1;
                    }
                } else if c == '"' {
                    mode = Mode::Normal;
                }
            }
            Mode::TemplateString => {
                out.push(c);
                i += 1;
                if c == '\\' {
                    if let Some(n) = next {
                        out.push(n);
                        i += 1;
                    }
                } else if c == '`' {
                    mode = Mode::Normal;
                }
            }
        }
    }

    out
}

fn rewrite_val_declarations(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("val ") || trimmed.starts_with("val\t") {
                let indent = &line[..line.len() - trimmed.len()];
                let rest = trimmed[3..].trim_start();
                format!("{}let {}", indent, rest)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_implements_blocks(source: &str) -> String {
    let mut out = Vec::new();
    let mut in_impl = false;
    let mut brace_depth: i32 = 0;

    for line in source.lines() {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];

        if !in_impl {
            if let Some(rest) = trimmed.strip_prefix("implements ") {
                if let Some((target, _)) = rest.split_once('{') {
                    let target = target.trim();
                    out.push(format!("{}class {} {{", indent, target));
                    in_impl = true;
                    brace_depth = 1;
                    continue;
                }
            }
            out.push(line.to_string());
            continue;
        }

        let rewritten = if let Some(rest) = trimmed.strip_prefix("function ") {
            format!("{}{}", indent, rest)
        } else {
            line.to_string()
        };

        brace_depth += rewritten.matches('{').count() as i32;
        brace_depth -= rewritten.matches('}').count() as i32;
        out.push(rewritten);

        if brace_depth <= 0 {
            in_impl = false;
            brace_depth = 0;
        }
    }

    out.join("\n")
}

pub fn compile_formatted(source: &str) -> Result<String> {
    compile(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_function() {
        let trust_code = r#"
            function add(a: number32, b: number32): number32 {
                return a + b;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("fn add"));
        assert!(result.contains("i32"));
    }

    #[test]
    fn test_compile_fibonacci() {
        let trust_code = r#"
            function fibonacci(n: number32): number32 {
                if (n <= 1) {
                    return n;
                }
                return fibonacci(n - 1) + fibonacci(n - 2);
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("fn fibonacci"));
        assert!(result.contains("i32"));
        assert!(result.contains("n <= 1"));
        assert!(result.contains("return n;"));
        assert!(result.contains("fibonacci(n - 1)"));
        assert!(result.contains("fibonacci(n - 2)"));
    }

    #[test]
    fn test_compile_struct() {
        let trust_code = r#"
            struct Point {
                x: number32;
                y: number32;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("struct Point"));
        assert!(result.contains("x: i32"));
        assert!(result.contains("y: i32"));
        assert!(result.contains("#[derive(Debug, Clone)]"));
    }

    #[test]
    fn test_compile_enum() {
        let trust_code = r#"
            enum Direction { North, South, East, West }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("enum Direction"));
        assert!(result.contains("North"));
        assert!(result.contains("West"));
    }

    #[test]
    fn test_compile_arrays() {
        let trust_code = r#"
            function test(arr: number32[]): number32 {
                let first = arr[0];
                arr.push(42);
                let n = arr.length;
                return first;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("Vec<i32>"));
        assert!(result.contains("arr[0 as usize]"));
        assert!(result.contains("arr.push(42)"));
        assert!(result.contains("arr.len()"));
    }

    #[test]
    fn test_compile_map() {
        let trust_code = r#"
            function test() {
                let m: Map<string, number32> = new Map();
                m.set("key", 1);
                let v = m.get("key");
                let exists = m.has("key");
                m.delete("key");
            }
        "#;
        let result = compile(trust_code).unwrap();
        assert!(result.contains("HashMap<String, i32>"));
        assert!(result.contains("HashMap::new()"));
        assert!(result.contains("m.insert("));
        assert!(result.contains("m.get(&"));
        assert!(result.contains("m.contains_key(&"));
        assert!(result.contains("m.remove(&"));
        assert!(result.contains("use std::collections::HashMap;"));
    }

    #[test]
    fn test_compile_set() {
        let trust_code = r#"
            function test() {
                let s: Set<string> = new Set();
                s.add("hello");
                let exists = s.has("hello");
                s.delete("hello");
            }
        "#;
        let result = compile(trust_code).unwrap();
        assert!(result.contains("HashSet<String>"));
        assert!(result.contains("HashSet::new()"));
        assert!(result.contains("s.insert("));
        assert!(result.contains("s.contains(&"));
        assert!(result.contains("s.remove(&"));
        assert!(result.contains("use std::collections::HashSet;"));
    }

    #[test]
    fn test_compile_throw() {
        let trust_code = r#"
            function divide(a: number32, b: number32): Result<number32, string> {
                if (b == 0) {
                    throw new Error("division by zero");
                }
                return ok(a / b);
            }
        "#;
        let result = compile(trust_code).unwrap();
        assert!(result.contains("return Err("));
        assert!(result.contains("division by zero"));
    }

    #[test]
    fn test_compile_string_enum() {
        let trust_code = r#"
            enum Status { Active = "active", Inactive = "inactive", Pending = "pending" }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("enum Status"));
        assert!(result.contains("Active,"));
        assert!(result.contains("fn as_str"));
        assert!(result.contains("Status::Active => \"active\""));
        assert!(result.contains("impl std::fmt::Display for Status"));
    }

    #[test]
    fn test_compile_string_case_methods() {
        let trust_code = r#"
            function normalize(name: string): string {
                let upper = name.toUpperCase();
                let lower = name.toLowerCase();
                return upper + lower;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("name.to_uppercase()"));
        assert!(result.contains("name.to_lowercase()"));
    }

    #[test]
    fn test_compile_string_substring() {
        let trust_code = r#"
            function cut(name: string): string {
                let a = name.substring(1);
                let b = name.substring(1, 3);
                return a + b;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("name.chars().collect()"));
        assert!(result.contains("__trust_start"));
        assert!(result.contains("__trust_end"));
        assert!(result.contains("collect::<String>()"));
    }

    #[test]
    fn test_compile_string_methods_extended() {
        let trust_code = r#"
            function normalize(name: string) {
                let starts = name.startsWith("A");
                let ends = name.endsWith("z");
                let hasX = name.includes("x");
                let idx = name.indexOf("x");
                let last = name.lastIndexOf("x");
                let one = name.charAt(0);
                let at = name.at(0);
                let sl = name.slice(1, 3);
                let sub = name.substr(1, 2);
                let replaced = name.replace("a", "b");
                let replacedAll = name.replaceAll("a", "b");
                let trimmed = name.trim();
                let left = name.trimStart();
                let right = name.trimEnd();
                let repeated = name.repeat(2);
                let parts = name.split(",");
                let full = name.concat("!", "?");
                return name.toUpperCase() + name.toLowerCase() + one + at + sl + sub + replaced + replacedAll + trimmed + left + right + repeated + full;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("name.starts_with((\"A\".to_string()).as_str())"));
        assert!(result.contains("name.ends_with((\"z\".to_string()).as_str())"));
        assert!(result.contains("name.contains((\"x\".to_string()).as_str())"));
        assert!(result.contains("match __trust_s.find((\"x\".to_string()).as_str())"));
        assert!(result.contains("char_indices().take_while(|(i, _)| *i < __trust_byte).count() as i32"));
        assert!(result.contains("match __trust_s.rfind((\"x\".to_string()).as_str())"));
        assert!(result.contains("name.replacen((\"a\".to_string()).as_str(), (\"b\".to_string()).as_str(), 1)"));
        assert!(result.contains("name.replace((\"a\".to_string()).as_str(), (\"b\".to_string()).as_str())"));
        assert!(result.contains("name.trim().to_string()"));
        assert!(result.contains("name.trim_start().to_string()"));
        assert!(result.contains("name.trim_end().to_string()"));
        assert!(result.contains("name.repeat((2).max(0) as usize)"));
        assert!(result.contains("name.split((\",\".to_string()).as_str()).map(|s| s.to_string()).collect::<Vec<String>>()"));
        assert!(result.contains("name.to_uppercase()"));
        assert!(result.contains("name.to_lowercase()"));
    }

    #[test]
    fn test_compile_pointer_string_methods() {
        let trust_code = r#"
            function test(p: Pointer<string>): string {
                let upper = p.toUpperCase();
                let sub = p.substring(1, 3);
                let has = p.includes("x");
                let len = p.length;
                return upper + sub;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("p.borrow().to_uppercase()"));
        assert!(result.contains("p.borrow().chars().collect()"));
        assert!(result.contains("p.borrow().contains((\"x\".to_string()).as_str())"));
        assert!(result.contains("p.borrow().chars().count() as i32"));
    }

    #[test]
    fn test_compile_builtin_cast_calls() {
        let trust_code = r#"
            function cast_all(a: int64, s: string, pn: Pointer<int32>, ps: Pointer<string>) {
                let n1: int32 = int32(a);
                let n2: int32 = int32(s);
                let n3: int32 = int32("42");
                let n4: int32 = int32(pn);
                let n5: int32 = int32(ps);
                let n6: int32 = number(s);
                let f: float64 = float64(a);
                let f2: float64 = float(a);
                let t1: string = string(a);
                let t2: string = string(ps);
                let t3: string = string(pn);
                let b1: boolean = boolean(a);
                let b2: boolean = boolean(s);
                let b3: boolean = boolean(ps);
                let b4: boolean = boolean(false);
                return t1;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("(a) as i32"));
        assert!(result.contains("(s).parse::<i32>().unwrap_or_default()"));
        assert!(result.contains("\"42\".to_string()).parse::<i32>().unwrap_or_default()"));
        assert!(result.contains("(*pn.borrow()) as i32"));
        assert!(result.contains("(ps.borrow()).parse::<i32>().unwrap_or_default()"));
        assert!(result.contains("(s).parse::<i32>().unwrap_or_default()"));
        assert!(result.contains("(a) as f64"));
        assert!(result.contains("(a) as f64"));
        assert!(result.contains("(a).to_string()"));
        assert!(result.contains("ps.borrow().to_string()"));
        assert!(result.contains("(*pn.borrow()).to_string()"));
        assert!(result.contains("(a) != 0"));
        assert!(result.contains("!(s).is_empty()"));
        assert!(result.contains("!(ps.borrow()).is_empty()"));
        assert!(result.contains("false"));
    }

    #[test]
    fn test_compile_val_var_and_global_const() {
        let trust_code = r#"
            const SCALE: int32 = 10;
            const APP: string = "TRUST";

            function test(): int32 {
                val x: int32 = 2;
                var y: int32 = 3;
                val label = APP;
                y = y + x;
                return y + SCALE;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("const SCALE: i32 = 10;"));
        assert!(result.contains("const APP: &'static str = \"TRUST\";"));
        assert!(result.contains("let x: i32 = 2;"));
        assert!(result.contains("let mut y: i32 = 3;"));
        assert!(result.contains("let label = APP;"));
        assert!(result.contains("y = y + x;"));
        assert!(result.contains("return y + SCALE;"));
    }

    #[test]
    fn test_compile_mod_and_exp() {
        let trust_code = r#"
            function ops(a: int32, b: int32, x: float64, y: float64): float64 {
                let m: int32 = a % b;
                let p: int32 = a ** 3;
                let q: float64 = x ** y;
                return q + float64(m) + float64(p);
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("a % b"));
        assert!(result.contains("(a as i32).pow((3).max(0) as u32)"));
        assert!(result.contains("(x as f64).powf(y as f64)"));
    }

    #[test]
    fn test_compile_word_boolean_ops_and_ternary() {
        let trust_code = r#"
            function test(a: boolean, b: boolean): int32 {
                // and or should not be rewritten inside comments
                let keep: string = "and or";
                let c: boolean = a and b or false;
                let n: int32 = c ? 1 : 2;
                return n;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("a && b || false"));
        assert!(result.contains("let keep: String = \"and or\".to_string();"));
        assert!(result.contains("let n: i32 = if c { 1 } else { 2 };"));
    }

    #[test]
    fn test_compile_for_and_loop_forms() {
        let trust_code = r#"
            function loops(arr: int32[]): int32 {
                var sum: int32 = 0;

                for (var i: int32 = 0; i < 3; i = i + 1) {
                    sum = sum + i;
                }

                for (item in arr) {
                    sum = sum + item;
                }

                for (item of arr) {
                    sum = sum + item;
                }

                loop (sum < 100 and sum >= 0) {
                    sum = sum + 1;
                }

                return sum;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("let mut i: i32 = 0;"));
        assert!(result.contains("while i < 3"));
        assert!(result.contains("i = i + 1;"));
        assert!(result.contains("for item in (arr).iter().cloned()"));
        assert!(result.contains("while sum < 100 && sum >= 0"));
    }

    #[test]
    fn test_compile_rejects_while_keyword() {
        let trust_code = r#"
            function bad(): int32 {
                var i: int32 = 0;
                while (i < 3) {
                    i = i + 1;
                }
                return i;
            }
        "#;

        let err = compile(trust_code).unwrap_err().to_string();
        assert!(err.contains("`while` is not supported in TRUST"));
    }

    #[test]
    fn test_compile_async_await_thread_model() {
        let trust_code = r#"
            async function compute(n: int32): int32 {
                return n + 1;
            }

            function main(): int32 {
                val handle = compute(41);
                val out = await handle;
                return out;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("fn compute(n: i32) -> std::thread::JoinHandle<i32>"));
        assert!(result.contains("std::thread::spawn(move || {"));
        assert!(result.contains("let out = (handle).join().unwrap();"));
    }

    #[test]
    fn test_compile_try_catch_finally() {
        let trust_code = r#"
            function safe_div(a: int32, b: int32): int32 {
                var out: int32 = 0;
                try {
                    if (b == 0) {
                        throw "division by zero";
                    }
                    out = a / b;
                } catch (e) {
                    console.write(e);
                    out = -1;
                } finally {
                    console.write("done");
                }
                return out;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("let __trust_try_result: Result<(), String>"));
        assert!(result.contains("if let Err(e) = __trust_try_result"));
        assert!(result.contains("return Err(\"division by zero\".to_string());"));
        assert!(result.contains("println!(\"{}\", e);"));
        assert!(result.contains("println!(\"{}\", \"done\".to_string());"));
    }

    #[test]
    fn test_compile_struct_constructor_call_style() {
        let trust_code = r#"
            struct Point {
                x: int32;
                y: int32;
            }

            function make(): Point {
                val p: Point = Point({ x: 1, y: 2 });
                return p;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("struct Point"));
        assert!(result.contains("let p: Point = Point { x: 1, y: 2 };"));
        assert!(result.contains("return p;"));
    }

    #[test]
    fn test_compile_import() {
        let trust_code = r#"
            import { Serialize, Deserialize } from "serde";
        "#;

        let output = compile_full(trust_code).unwrap();
        assert!(output.rust_code.contains("use serde::{Serialize, Deserialize};"));
        assert!(output.required_crates.contains(&"serde".to_string()));
    }

    #[test]
    fn test_compile_trusty_json_struct_serde() {
        let trust_code = r#"
            import { toJSON, fromJSON } from "trusty:json";

            struct User {
                id: int32;
                name: string;
            }

            function roundtrip(): string {
                val u: User = User({ id: 1, name: "Alice" });
                val json = toJSON(u);
                val u2: User = fromJSON(json);
                return u2.name;
            }
        "#;

        let output = compile_full(trust_code).unwrap();
        assert!(output
            .rust_code
            .contains("#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]"));
        assert!(output.rust_code.contains("pub fn toJSON<T: serde::Serialize>(value: T) -> String"));
        assert!(output
            .rust_code
            .contains("pub fn fromJSON<T: serde::de::DeserializeOwned>(json: String) -> T"));
        assert!(output.required_crates.contains(&"serde".to_string()));
        assert!(output.required_crates.contains(&"serde_derive".to_string()));
        assert!(output.required_crates.contains(&"serde_json".to_string()));
    }

    #[test]
    fn test_compile_string_literal_escapes_quotes() {
        let trust_code = r#"
            function main() {
                val s = "{\"ok\":true,\"count\":2}";
                console.write(s);
            }
        "#;

        let output = compile(trust_code).unwrap();
        assert!(output.contains("\"{\\\"ok\\\":true,\\\"count\\\":2}\".to_string()"));
    }

    #[test]
    fn test_compile_implements_block() {
        let trust_code = r#"
            struct User {
                name: string;
            }

            implements User {
                function greet(): string {
                    return this.name.toUpperCase();
                }

                function rename(newName: string): void {
                    this.name = newName;
                }
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("impl User {"));
        assert!(result.contains("fn greet(&self) -> String"));
        assert!(result.contains("self.name.to_uppercase()"));
        assert!(result.contains("fn rename(&mut self, newName: String) -> ()"));
        assert!(result.contains("self.name = newName;"));
    }

    #[test]
    fn test_compile_trusty_time_light_date_fns_helpers() {
        let trust_code = r#"
            import { DateTime, compare, addDays, addMonths, addYears, addMinutes, addSeconds, subDays, subMonths, subYears, subMinutes, subSeconds } from "trusty:time";

            function demo(): int32 {
                val now = DateTime.now();
                val a = addDays(now, 2);
                val b = addMinutes(a, 30);
                val c = addSeconds(b, 15);
                val d = addMonths(c, 2);
                val e = addYears(d, 1);
                val f = subDays(e, 1);
                val g = subMonths(f, 1);
                val h = subYears(g, 1);
                val i = subMinutes(h, 10);
                val j = subSeconds(i, 5);
                return compare(now, j);
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("use std::time::{Instant, Duration, SystemTime as RustSystemTime};"));
        assert!(result.contains("fn compare(a: DateTime, b: DateTime) -> i32"));
        assert!(result.contains("fn addDays(dateTime: DateTime, days: i32) -> DateTime"));
        assert!(result.contains("fn addMonths(dateTime: DateTime, months: i32) -> DateTime"));
        assert!(result.contains("fn addYears(dateTime: DateTime, years: i32) -> DateTime"));
        assert!(result.contains("fn subSeconds(dateTime: DateTime, seconds: i32) -> DateTime"));
        assert!(result.contains("fn subMonths(dateTime: DateTime, months: i32) -> DateTime"));
        assert!(result.contains("fn subYears(dateTime: DateTime, years: i32) -> DateTime"));
        assert!(result.contains("let now = DateTime::now();"));
        assert!(result.contains("let a = addDays(now, 2);"));
        assert!(result.contains("let d = addMonths(c, 2);"));
        assert!(result.contains("let e = addYears(d, 1);"));
        assert!(result.contains("let g = subMonths(f, 1);"));
        assert!(result.contains("let h = subYears(g, 1);"));
        assert!(result.contains("let j = subSeconds(i, 5);"));
        assert!(result.contains("return compare(now, j);"));
    }

    #[test]
    fn test_compile_trusty_time_date_time_datetime_helpers() {
        let trust_code = r#"
            import { Date, Time, DateTime } from "trusty:time";

            function demo(): string {
                val d = Date.fromYmd(2026, 2, 15).addMonths(1).addYears(1).subMonths(1).subYears(1).addDays(3);
                val t = Time.fromHmsMilli(10, 30, 0, 250).subMinutes(45);
                val dt = DateTime.fromParts(d, t).addMonths(2).addYears(1).subMonths(1).subYears(1).addHours(2).startOfDay();
                val ds = d.toIsoString();
                val ts = t.toIsoString();
                val dts = dt.toIsoString();
                return `${ds}|${ts}|${dts}`;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("pub struct Date"));
        assert!(result.contains("pub struct Time"));
        assert!(result.contains("pub struct DateTime"));
        assert!(result.contains("let d = Date::fromYmd(2026, 2, 15).addMonths(1).addYears(1).subMonths(1).subYears(1).addDays(3);"));
        assert!(result.contains("let t = Time::fromHmsMilli(10, 30, 0, 250).subMinutes(45);"));
        assert!(result.contains("let dt = DateTime::fromParts(d, t).addMonths(2).addYears(1).subMonths(1).subYears(1).addHours(2).startOfDay();"));
        assert!(result.contains("let ds = d.toIsoString();"));
        assert!(result.contains("let ts = t.toIsoString();"));
        assert!(result.contains("let dts = dt.toIsoString();"));
        assert!(result.contains("return format!(\"{}|{}|{}\", ds, ts, dts);"));
    }

    #[test]
    fn test_compile_trusty_math_helpers() {
        let trust_code = r#"
            import { sqrt, pow, log, abs, min, max, clamp, sin, cos, tan, PI, E } from "trusty:math";

            function demo(x: int32): float64 {
                val a = sqrt(x);
                val p = pow(2, 8);
                val lg = log(100.0);
                val lgb = log(8.0, 2.0);
                val b = abs(-42);
                val c = min(10, 20);
                val d = max(10, 20);
                val e = clamp(x, 0, 100);
                val f = sin(1.0) + cos(1.0) + tan(1.0) + PI + E;
                return a + p + lg + lgb + float64(b + c + d + e) + f;
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("pub const PI: f64 = std::f64::consts::PI;"));
        assert!(result.contains("pub const E: f64 = std::f64::consts::E;"));
        assert!(result.contains("pub fn sqrt<T: Into<f64>>(x: T) -> f64"));
        assert!(result.contains("pub fn pow<A: Into<f64>, B: Into<f64>>(base: A, exp: B) -> f64"));
        assert!(result.contains("pub fn log<T: Into<f64>>(value: T) -> f64"));
        assert!(result.contains("pub fn log_base<V: Into<f64>, B: Into<f64>>(value: V, base: B) -> f64"));
        assert!(result.contains("pub fn abs<T: __TrustMathAbs>(x: T) -> T"));
        assert!(result.contains("pub fn min<T: PartialOrd + Copy>(a: T, b: T) -> T"));
        assert!(result.contains("pub fn max<T: PartialOrd + Copy>(a: T, b: T) -> T"));
        assert!(result.contains("pub fn clamp<T: PartialOrd + Copy>(x: T, lo: T, hi: T) -> T"));
        assert!(result.contains("pub fn sin<T: Into<f64>>(x: T) -> f64"));
        assert!(result.contains("let a = sqrt(x);"));
        assert!(result.contains("let p = pow(2, 8);"));
        assert!(result.contains("let lg = log("));
        assert!(result.contains("let lgb = log_base("));
        assert!(result.contains("let e = clamp(x, 0, 100);"));
    }

    #[test]
    fn test_compile_trusty_math_default_alias_namespace_style() {
        let trust_code = r#"
            import math from "trusty:math";

            function demo(): float64 {
                return math.PI + math.E + math.sqrt(9) + math.pow(2, 8) + math.log(10.0) + math.log(8.0, 2.0);
            }
        "#;

        let result = compile(trust_code).unwrap();
        assert!(result.contains("mod __trusty_math {"));
        assert!(result.contains("use __trusty_math as math;"));
        assert!(result.contains("math::log("));
        assert!(result.contains("math::log_base("));
    }

    #[test]
    fn test_compile_trusty_rand_helpers() {
        let trust_code = r#"
            import { random, randomInt, randomFloat, bernoulli, weightedIndex } from "trusty:rand";

            function demo(): int32 {
                val a = random();
                val b = randomInt(1, 6);
                val c = randomFloat(0.0, 1.0);
                val d = bernoulli(0.5);
                val i = weightedIndex([0.2, 0.3, 0.5]);
                return b + i + int32(c) + int32(a) + int32(boolean(d));
            }
        "#;

        let output = compile_full(trust_code).unwrap();
        let result = output.rust_code;
        assert!(result.contains("use rand::Rng;"));
        assert!(result.contains("use rand::distributions::{Bernoulli, Distribution, WeightedIndex};"));
        assert!(result.contains("pub fn random() -> f64"));
        assert!(result.contains("pub fn randomInt(min: i32, max: i32) -> i32"));
        assert!(result.contains("pub fn randomFloat(min: f64, max: f64) -> f64"));
        assert!(result.contains("pub fn bernoulli(p: f64) -> bool"));
        assert!(result.contains("pub fn weightedIndex(weights: Vec<f64>) -> i32"));
        assert!(output.required_crates.contains(&"rand".to_string()));
    }
}
