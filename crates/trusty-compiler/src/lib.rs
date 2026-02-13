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
    let ast = parser::parse_typescript(source)?;
    transpiler::transpile_to_rust(&ast)
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
    fn test_compile_import() {
        let trust_code = r#"
            import { Serialize, Deserialize } from "serde";
        "#;

        let output = compile_full(trust_code).unwrap();
        assert!(output.rust_code.contains("use serde::{Serialize, Deserialize};"));
        assert!(output.required_crates.contains(&"serde".to_string()));
    }
}
