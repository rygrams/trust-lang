pub mod enums;
pub mod expressions;
pub mod functions;
pub mod imports;
pub mod scope;
pub mod statements;
pub mod structs;
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
    let mut type_decls: Vec<String> = Vec::new(); // structs + enums
    let mut impl_blocks: Vec<String> = Vec::new();
    let mut global_consts: Vec<String> = Vec::new();
    let mut function_code: Vec<String> = Vec::new();
    let mut required_crates: Vec<String> = Vec::new();
    let mut module_aliases: Vec<String> = Vec::new();

    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                let info = imports::transpile_import(import_decl)?;
                for stmt in info.use_statements {
                    if !use_statements.contains(&stmt) {
                        use_statements.push(stmt);
                    }
                }
                for name in info.required_crates {
                    if !required_crates.contains(&name) {
                        required_crates.push(name);
                    }
                }
                for alias in info.module_aliases {
                    if !module_aliases.contains(&alias) {
                        module_aliases.push(alias);
                    }
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(interface_decl))) => {
                let struct_code = structs::transpile_interface(interface_decl)?;
                type_decls.push(struct_code);
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::TsEnum(enum_decl))) => {
                let enum_code = enums::transpile_enum(enum_decl)?;
                type_decls.push(enum_code);
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(func_decl))) => {
                let func_code = functions::transpile_function(func_decl, &module_aliases)?;
                function_code.push(func_code);
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Class(class_decl))) => {
                if let Some(impl_code) = functions::transpile_impl_block(class_decl, &module_aliases)? {
                    impl_blocks.push(impl_code);
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                global_consts.extend(statements::transpile_global_const(var_decl)?);
            }
            _ => {}
        }
    }

    let all_code: String = use_statements
        .iter()
        .chain(type_decls.iter())
        .chain(impl_blocks.iter())
        .chain(global_consts.iter())
        .chain(function_code.iter())
        .cloned()
        .collect();

    // Auto-inject Rc/RefCell if Pointer<T> is used
    if all_code.contains("Rc<RefCell<") {
        if !use_statements.contains(&"use std::rc::Rc;".to_string()) {
            use_statements.insert(0, "use std::rc::Rc;".to_string());
        }
        if !use_statements.contains(&"use std::cell::RefCell;".to_string()) {
            use_statements.insert(1, "use std::cell::RefCell;".to_string());
        }
    }

    // Auto-inject Arc/Mutex if Threaded<T> is used
    if all_code.contains("Arc<Mutex<") {
        if !use_statements.contains(&"use std::sync::{Arc, Mutex};".to_string()) {
            use_statements.insert(0, "use std::sync::{Arc, Mutex};".to_string());
        }
    }

    // Auto-inject HashMap if Map<> is used
    if all_code.contains("HashMap<") || all_code.contains("HashMap::new()") {
        if !use_statements.contains(&"use std::collections::HashMap;".to_string()) {
            use_statements.insert(0, "use std::collections::HashMap;".to_string());
        }
    }

    // Auto-inject HashSet if Set<> is used
    if all_code.contains("HashSet<") || all_code.contains("HashSet::new()") {
        if !use_statements.contains(&"use std::collections::HashSet;".to_string()) {
            use_statements.insert(0, "use std::collections::HashSet;".to_string());
        }
    }

    let mut rust_code = String::new();

    for stmt in &use_statements {
        rust_code.push_str(stmt);
        rust_code.push('\n');
    }
    if !use_statements.is_empty() {
        rust_code.push('\n');
    }
    for decl in &type_decls {
        rust_code.push_str(decl);
        rust_code.push_str("\n\n");
    }
    for block in &impl_blocks {
        rust_code.push_str(block);
        rust_code.push_str("\n\n");
    }
    for global_const in &global_consts {
        rust_code.push_str(global_const);
        rust_code.push_str("\n\n");
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
