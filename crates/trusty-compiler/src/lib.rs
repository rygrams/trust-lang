pub mod codegen;
pub mod parser;
pub mod transpiler;

use anyhow::Result;

pub fn compile(source: &str) -> Result<String> {
    let ast = parser::parse_typescript(source)?;

    let rust_code = transpiler::transpile_to_rust(&ast)?;

    Ok(rust_code)
}

pub fn compile_formatted(source: &str) -> Result<String> {
    let rust_code = compile(source)?;

    Ok(rust_code)
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
}
