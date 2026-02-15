use crate::stdlib;
use anyhow::Result;
use swc_ecma_ast::*;

pub struct ImportInfo {
    /// One or more `use …;` lines to inject into the generated Rust file.
    pub use_statements: Vec<String>,
    /// External crate name(s) that must appear in Cargo.toml (empty for std / trusty:* std-only).
    pub required_crates: Vec<String>,
}

pub fn transpile_import(import: &ImportDecl) -> Result<ImportInfo> {
    let src = import.src.value.to_string_lossy();

    // ── trusty:* standard library ────────────────────────────────────────────
    if let Some(module_name) = src.strip_prefix("trusty:") {
        if let Some(stdlib_mod) = stdlib::resolve(module_name) {
            return Ok(ImportInfo {
                use_statements: stdlib_mod.use_statements,
                required_crates: stdlib_mod
                    .required_crates
                    .into_iter()
                    .map(|(name, _version)| name)
                    .collect(),
            });
        }
        // Unknown trusty: module — emit a comment so the user knows
        return Ok(ImportInfo {
            use_statements: vec![format!("// trusty:{} — module not yet implemented", module_name)],
            required_crates: vec![],
        });
    }

    // ── Local .trs imports (./foo, ../bar) — already bundled by the CLI ──────
    if src.starts_with('.') {
        return Ok(ImportInfo {
            use_statements: vec![],
            required_crates: vec![],
        });
    }

    // ── Raw external Rust crate ───────────────────────────────────────────────
    // "serde" → "serde", "std/collections" → "std::collections"
    let module_path = src.replace('/', "::");

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
        use_statements: vec![use_statement],
        required_crates: crate_name.into_iter().collect(),
    })
}
