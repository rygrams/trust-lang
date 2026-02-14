use super::expressions::transpile_expression;
use super::scope::{is_pointer, is_threaded, Scope};
use super::types::transpile_type_annotation;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_statement(stmt: &Stmt, scope: &mut Scope) -> Result<String> {
    match stmt {
        Stmt::Return(return_stmt) => {
            if let Some(arg) = &return_stmt.arg {
                let expr = transpile_expression(arg, scope)?;
                Ok(format!("return {};", expr))
            } else {
                Ok("return;".to_string())
            }
        }
        Stmt::Expr(expr_stmt) => {
            let expr = transpile_expression(&expr_stmt.expr, scope)?;
            Ok(format!("{};", expr))
        }
        Stmt::Block(block_stmt) => transpile_block_stmt(block_stmt, "    ", scope),
        Stmt::If(if_stmt) => {
            let cond = transpile_expression(&if_stmt.test, scope)?;
            let cons = transpile_statement(&if_stmt.cons, scope)?;
            if let Some(alt) = &if_stmt.alt {
                let alt_str = transpile_statement(alt, scope)?;
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
                        .map(|ann| transpile_type_annotation(ann)),
                    _ => None,
                };

                let declared_as_pointer = type_ann.as_ref().map(|t| is_pointer(t)).unwrap_or(false);
                let declared_as_threaded = type_ann.as_ref().map(|t| is_threaded(t)).unwrap_or(false);

                if let Some(init) = &decl.init {
                    // `let p2 = p` where p is already a Pointer or Threaded → clone
                    let init_shared_name = match &**init {
                        Expr::Ident(ident) => {
                            let n = ident.sym.to_string();
                            let ty = scope.get(&n).map(|t| t.as_str()).unwrap_or("");
                            if is_pointer(ty) || is_threaded(ty) { Some(n) } else { None }
                        }
                        _ => None,
                    };

                    let val = if declared_as_pointer {
                        // `let p: Pointer<T> = expr` → Rc::new(RefCell::new(expr))
                        let expr_str = transpile_expression(init, scope)?;
                        scope.insert(name.clone(), type_ann.clone().unwrap());
                        format!("Rc::new(RefCell::new({}))", expr_str)
                    } else if declared_as_threaded {
                        // `let s: Threaded<T> = expr` → Arc::new(Mutex::new(expr))
                        let expr_str = transpile_expression(init, scope)?;
                        scope.insert(name.clone(), type_ann.clone().unwrap());
                        format!("Arc::new(Mutex::new({}))", expr_str)
                    } else if let Some(src) = &init_shared_name {
                        // `let p2 = p` → clone, inherit type
                        let shared_type = scope.get(src).cloned().unwrap();
                        let clone_fn = if is_threaded(&shared_type) { "Arc::clone" } else { "Rc::clone" };
                        scope.insert(name.clone(), shared_type);
                        format!("{}(&{})", clone_fn, src)
                    } else {
                        transpile_expression(init, scope)?
                    };

                    match &type_ann {
                        Some(ty) => parts.push(format!("let {}: {} = {};", name, ty, val)),
                        None => parts.push(format!("let {} = {};", name, val)),
                    }
                }
            }
            Ok(parts.join("\n"))
        }
        _ => Ok("// Statement non supporté".to_string()),
    }
}

pub fn transpile_block_stmt(block: &BlockStmt, indent: &str, scope: &mut Scope) -> Result<String> {
    let mut result = Vec::new();
    for s in &block.stmts {
        let stmt_str = transpile_statement(s, scope)?;
        result.push(format!("{}{}", indent, stmt_str));
    }
    Ok(result.join("\n"))
}
