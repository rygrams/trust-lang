pub mod codegen;
pub mod parser;
pub mod transpiler;

use anyhow::Result;

pub use transpiler::TranspileOutput;

/// Transpile TRUST source to Rust source code.
pub fn compile(source: &str) -> Result<String> {
    Ok(compile_full(source)?.rust_code)
}

/// Transpile TRUST source and return Rust code + required external crates.
pub fn compile_full(source: &str) -> Result<TranspileOutput> {
    let preprocessed = preprocess(source);
    let ast = parser::parse_typescript(&preprocessed)?;
    transpiler::transpile_to_rust(&ast)
}

/// Rewrite TRUST-specific keywords to valid TypeScript before SWC parsing.
fn preprocess(source: &str) -> String {
    source
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
    fn test_compile_import() {
        let trust_code = r#"
            import { Serialize, Deserialize } from "serde";
        "#;

        let output = compile_full(trust_code).unwrap();
        assert!(output.rust_code.contains("use serde::{Serialize, Deserialize};"));
        assert!(output.required_crates.contains(&"serde".to_string()));
    }
}
