pub mod expressions;
pub mod functions;
pub mod imports;
pub mod statements;
pub mod types;

use anyhow::Result;
use swc_ecma_ast::*;

pub struct TranspileOutput {
    pub rust_code: String,
    /// External crate names required (from import declarations)
    pub required_crates: Vec<String>,
}

pub fn transpile_to_rust(module: &Module) -> Result<TranspileOutput> {
    let mut use_statements: Vec<String> = Vec::new();
    let mut function_code: Vec<String> = Vec::new();
    let mut required_crates: Vec<String> = Vec::new();

    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                let info = imports::transpile_import(import_decl)?;
                use_statements.push(info.use_statement);
                if let Some(name) = info.crate_name {
                    if !required_crates.contains(&name) {
                        required_crates.push(name);
                    }
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(func_decl))) => {
                let func_code = functions::transpile_function(func_decl)?;
                function_code.push(func_code);
            }
            _ => {}
        }
    }

    let mut rust_code = String::new();

    for stmt in &use_statements {
        rust_code.push_str(stmt);
        rust_code.push('\n');
    }
    if !use_statements.is_empty() && !function_code.is_empty() {
        rust_code.push('\n');
    }
    for func in &function_code {
        rust_code.push_str(func);
        rust_code.push_str("\n\n");
    }

    Ok(TranspileOutput {
        rust_code: rust_code.trim().to_string(),
        required_crates,
    })
}
