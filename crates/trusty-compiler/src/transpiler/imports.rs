use anyhow::Result;
use swc_ecma_ast::*;

pub struct ImportInfo {
    /// e.g. `use serde::{Serialize, Deserialize};`
    pub use_statement: String,
    /// Crate name if it needs a Cargo dependency (None for std/core/alloc)
    pub crate_name: Option<String>,
}

pub fn transpile_import(import: &ImportDecl) -> Result<ImportInfo> {
    // "serde" → "serde", "std/collections" → "std::collections"
    let module_path = import.src.value.to_string_lossy().replace('/', "::");

    let names: Vec<String> = import
        .specifiers
        .iter()
        .map(|spec| match spec {
            ImportSpecifier::Named(named) => named.local.sym.to_string(),
            ImportSpecifier::Default(def) => def.local.sym.to_string(),
            ImportSpecifier::Namespace(ns) => format!("* as {}", ns.local.sym),
        })
        .collect();

    let use_statement = if names.is_empty() {
        format!("use {};", module_path)
    } else if names.len() == 1 {
        format!("use {}::{};", module_path, names[0])
    } else {
        format!("use {}::{{{}}};", module_path, names.join(", "))
    };

    let top_level = module_path.split("::").next().unwrap_or("").to_string();

    let crate_name = match top_level.as_str() {
        "std" | "core" | "alloc" => None,
        name => Some(name.to_string()),
    };

    Ok(ImportInfo {
        use_statement,
        crate_name,
    })
}
