use super::expressions::*;
use super::types::transpile_type_annotation;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_statement(stmt: &Stmt) -> Result<String> {
    match stmt {
        Stmt::Return(return_stmt) => {
            if let Some(arg) = &return_stmt.arg {
                let expr = transpile_expression(arg)?;
                Ok(format!("return {};", expr))
            } else {
                Ok("return;".to_string())
            }
        }
        Stmt::Expr(expr_stmt) => {
            let expr = transpile_expression(&expr_stmt.expr)?;
            Ok(format!("{};", expr))
        }
        Stmt::Block(block_stmt) => transpile_block_stmt(block_stmt, "    "),
        Stmt::If(if_stmt) => {
            let cond = transpile_expression(&if_stmt.test)?;
            let cons = transpile_statement(&if_stmt.cons)?;
            if let Some(alt) = &if_stmt.alt {
                let alt_str = transpile_statement(alt)?;
                Ok(format!("if {} {{\n{}\n}} else {{\n{}\n}}", cond, cons, alt_str))
            } else {
                Ok(format!("if {} {{\n{}\n}}", cond, cons))
            }
        }
        Stmt::Decl(Decl::Var(var_decl)) => {
            let mut parts = Vec::new();
            for decl in &var_decl.decls {
                let name = match &decl.name {
                    Pat::Ident(ident) => ident.id.sym.to_string(),
                    _ => "unknown".to_string(),
                };
                let type_ann = match &decl.name {
                    Pat::Ident(ident) => ident
                        .type_ann
                        .as_deref()
                        .map(|ann| format!(": {}", transpile_type_annotation(ann))),
                    _ => None,
                };
                if let Some(init) = &decl.init {
                    let val = transpile_expression(init)?;
                    match type_ann {
                        Some(ty) => parts.push(format!("let {}{} = {};", name, ty, val)),
                        None => parts.push(format!("let {} = {};", name, val)),
                    }
                }
            }
            Ok(parts.join("\n"))
        }
        _ => Ok("// Statement non supporté".to_string()),
    }
}

pub fn transpile_block_stmt(block: &BlockStmt, indent: &str) -> Result<String> {
    let stmts: Result<Vec<String>> = block
        .stmts
        .iter()
        .map(|s| transpile_statement(s).map(|r| format!("{}{}", indent, r)))
        .collect();
    Ok(stmts?.join("\n"))
}
