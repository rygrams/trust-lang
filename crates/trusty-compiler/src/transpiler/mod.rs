pub mod expressions;
pub mod functions;
pub mod statements;
pub mod types;

use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_to_rust(module: &Module) -> Result<String> {
    let mut rust_code = String::new();

    for item in &module.body {
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(func_decl))) = item {
            let func_code = functions::transpile_function(func_decl)?;
            rust_code.push_str(&func_code);
            rust_code.push_str("\n\n");
        }
    }

    Ok(rust_code.trim().to_string())
}
