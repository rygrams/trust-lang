use crate::stdlib;
use anyhow::{bail, Result};
use swc_ecma_ast::*;

pub struct ImportInfo {
    /// One or more `use …;` lines to inject into the generated Rust file.
    pub use_statements: Vec<String>,
    /// External crate name(s) that must appear in Cargo.toml (empty for std / trusty:* std-only).
    pub required_crates: Vec<String>,
    /// Names that should be treated as module aliases (`alias.member` => `alias::member` in Rust).
    pub module_aliases: Vec<String>,
}

pub fn transpile_import(import: &ImportDecl) -> Result<ImportInfo> {
    let src = import.src.value.to_string_lossy();
    let default_alias = import.specifiers.iter().find_map(|spec| match spec {
        ImportSpecifier::Default(def) => Some(def.local.sym.to_string()),
        _ => None,
    });
    let has_non_default_specifier = import
        .specifiers
        .iter()
        .any(|spec| !matches!(spec, ImportSpecifier::Default(_)));

    // ── trusty:* standard library ────────────────────────────────────────────
    if let Some(module_name) = src.strip_prefix("trusty:") {
        if default_alias.is_some() && has_non_default_specifier {
            bail!("Mixed default + named imports are not supported for trusty:* modules.");
        }
        if let Some(stdlib_mod) = stdlib::resolve(module_name) {
            if let Some(alias) = default_alias {
                if module_name != "math" {
                    bail!(
                        "Default import alias is currently supported only for \"trusty:math\"."
                    );
                }
                let wrapped_module = format!(
                    "mod __trusty_{module_name} {{\n{}\n}}\nuse __trusty_{module_name} as {alias};",
                    stdlib_mod.use_statements.join("\n")
                );
                return Ok(ImportInfo {
                    use_statements: vec![wrapped_module],
                    required_crates: stdlib_mod
                        .required_crates
                        .into_iter()
                        .map(|(name, _version)| name)
                        .collect(),
                    module_aliases: vec![alias],
                });
            }
            return Ok(ImportInfo {
                use_statements: stdlib_mod.use_statements,
                required_crates: stdlib_mod
                    .required_crates
                    .into_iter()
                    .map(|(name, _version)| name)
                    .collect(),
                module_aliases: vec![],
            });
        }
        // Unknown trusty: module — emit a comment so the user knows
        return Ok(ImportInfo {
            use_statements: vec![format!("// trusty:{} — module not yet implemented", module_name)],
            required_crates: vec![],
            module_aliases: vec![],
        });
    }

    // ── Local .trs imports (./foo, ../bar) — already bundled by the CLI ──────
    if src.starts_with('.') {
        if default_alias.is_some() {
            bail!("Default import aliases are not supported for local TRUST modules yet.");
        }
        return Ok(ImportInfo {
            use_statements: vec![],
            required_crates: vec![],
            module_aliases: vec![],
        });
    }

    // ── Raw external Rust crate ───────────────────────────────────────────────
    // "serde" → "serde", "std/collections" → "std::collections"
    let module_path = src.replace('/', "::");

    if let Some(alias) = default_alias {
        if has_non_default_specifier {
            bail!("Mixed default + named imports are not supported for external crates.");
        }
        let top_level = module_path.split("::").next().unwrap_or("").to_string();
        let crate_name = match top_level.as_str() {
            "std" | "core" | "alloc" => None,
            name => Some(name.to_string()),
        };
        return Ok(ImportInfo {
            use_statements: vec![format!("use {} as {};", module_path, alias)],
            required_crates: crate_name.into_iter().collect(),
            module_aliases: vec![alias],
        });
    }

    let names: Vec<String> = import
        .specifiers
        .iter()
        .map(|spec| match spec {
            ImportSpecifier::Named(named) => named.local.sym.to_string(),
            ImportSpecifier::Default(_) => unreachable!("handled above"),
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
        module_aliases: vec![],
    })
}
