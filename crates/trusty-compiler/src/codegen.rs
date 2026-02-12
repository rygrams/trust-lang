use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn write_rust_file(rust_code: &str, output_path: &Path) -> Result<()> {
    fs::write(output_path, rust_code)?;
    Ok(())
}

pub fn format_rust_code(code: &str) -> String {
    code.to_string()
}
