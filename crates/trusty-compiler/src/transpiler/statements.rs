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
        Stmt::While(while_stmt) => transpile_while_stmt(while_stmt, scope),
        Stmt::For(for_stmt) => transpile_for_stmt(for_stmt, scope),
        Stmt::ForIn(for_in_stmt) => transpile_for_in_stmt(for_in_stmt, scope),
        Stmt::ForOf(for_of_stmt) => transpile_for_of_stmt(for_of_stmt, scope),
        Stmt::Try(try_stmt) => transpile_try_stmt(try_stmt, scope),
        Stmt::Break(_) => Ok("break;".to_string()),
        Stmt::Continue(_) => Ok("continue;".to_string()),
        Stmt::Decl(Decl::Var(var_decl)) => {
            let is_mut = matches!(var_decl.kind, VarDeclKind::Var);
            let binding = if is_mut { "let mut" } else { "let" };
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
                        let expr_str = transpile_expression(init, scope)?;
                        // Register all typed variables in scope for method dispatch
                        if let Some(ty) = &type_ann {
                            scope.insert(name.clone(), ty.clone());
                        }
                        expr_str
                    };

                    match &type_ann {
                        Some(ty) => parts.push(format!("{} {}: {} = {};", binding, name, ty, val)),
                        None => parts.push(format!("{} {} = {};", binding, name, val)),
                    }
                }
            }
            Ok(parts.join("\n"))
        }
        Stmt::Throw(throw_stmt) => {
            // throw new Error("msg") → return Err("msg".to_string())
            let msg = match &*throw_stmt.arg {
                Expr::New(new_expr) => {
                    if let Some(args) = &new_expr.args {
                        if let Some(first) = args.first() {
                            transpile_expression(&first.expr, scope)?
                        } else {
                            "\"error\".to_string()".to_string()
                        }
                    } else {
                        "\"error\".to_string()".to_string()
                    }
                }
                other => transpile_expression(other, scope)?,
            };
            Ok(format!("return Err({});", msg))
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

pub fn transpile_global_const(var_decl: &VarDecl) -> Result<Vec<String>> {
    if !matches!(var_decl.kind, VarDeclKind::Const) {
        return Ok(Vec::new());
    }

    let scope = Scope::new();
    let mut parts = Vec::new();
    for decl in &var_decl.decls {
        let (name, type_ann) = match &decl.name {
            Pat::Ident(ident) => (
                ident.id.sym.to_string(),
                ident
                    .type_ann
                    .as_deref()
                    .map(|ann| transpile_type_annotation(ann)),
            ),
            _ => continue,
        };

        let Some(init) = &decl.init else {
            continue;
        };
        let val = transpile_const_value(init, &scope)?;

        match type_ann {
            Some(ty) if ty == "String" => parts.push(format!("const {}: &'static str = {};", name, val)),
            Some(ty) => parts.push(format!("const {}: {} = {};", name, ty, val)),
            None => parts.push(format!("const {}: i32 = {};", name, val)),
        }
    }
    Ok(parts)
}

fn transpile_const_value(expr: &Expr, scope: &Scope) -> Result<String> {
    match expr {
        Expr::Lit(Lit::Str(s)) => Ok(format!("\"{}\"", s.value.to_string_lossy())),
        Expr::Lit(Lit::Num(n)) => Ok(n.value.to_string()),
        Expr::Lit(Lit::Bool(b)) => Ok(b.value.to_string()),
        Expr::Unary(unary) if matches!(unary.op, UnaryOp::Minus) => {
            let inner = transpile_const_value(&unary.arg, scope)?;
            Ok(format!("-{}", inner))
        }
        _ => transpile_expression(expr, scope),
    }
}

fn transpile_while_stmt(while_stmt: &WhileStmt, scope: &mut Scope) -> Result<String> {
    let cond = transpile_expression(&while_stmt.test, scope)?;
    let body = transpile_statement(&while_stmt.body, scope)?;
    Ok(format!("while {} {{\n{}\n}}", cond, indent_block(&body, "    ")))
}

fn transpile_for_stmt(for_stmt: &ForStmt, scope: &mut Scope) -> Result<String> {
    let init = match &for_stmt.init {
        Some(VarDeclOrExpr::VarDecl(var_decl)) => {
            transpile_statement(&Stmt::Decl(Decl::Var(Box::new((**var_decl).clone()))), scope)?
        }
        Some(VarDeclOrExpr::Expr(expr)) => format!("{};", transpile_expression(expr, scope)?),
        None => String::new(),
    };

    let cond = match &for_stmt.test {
        Some(test) => transpile_expression(test, scope)?,
        None => "true".to_string(),
    };

    let update = match &for_stmt.update {
        Some(update) => Some(format!("{};", transpile_expression(update, scope)?)),
        None => None,
    };

    let body = transpile_statement(&for_stmt.body, scope)?;
    let mut while_body = indent_block(&body, "    ");
    if let Some(update) = update {
        if !while_body.is_empty() {
            while_body.push('\n');
        }
        while_body.push_str("    ");
        while_body.push_str(&update);
    }
    let while_code = format!("while {} {{\n{}\n}}", cond, while_body);

    if init.is_empty() {
        Ok(while_code)
    } else {
        Ok(format!("{}\n{}", init, while_code))
    }
}

fn transpile_for_in_stmt(for_in: &ForInStmt, scope: &mut Scope) -> Result<String> {
    let (binding, prelude) = transpile_for_head_binding(&for_in.left, scope)?;
    let right = transpile_expression(&for_in.right, scope)?;
    let body = transpile_statement(&for_in.body, scope)?;
    let for_code = format!(
        "for {} in ({}).iter().cloned() {{\n{}\n}}",
        binding,
        right,
        indent_block(&body, "    ")
    );
    if prelude.is_empty() {
        Ok(for_code)
    } else {
        Ok(format!("{}\n{}", prelude, for_code))
    }
}

fn transpile_for_of_stmt(for_of: &ForOfStmt, scope: &mut Scope) -> Result<String> {
    let (binding, prelude) = transpile_for_head_binding(&for_of.left, scope)?;
    let right = transpile_expression(&for_of.right, scope)?;
    let body = transpile_statement(&for_of.body, scope)?;
    let for_code = format!(
        "for {} in ({}).iter().cloned() {{\n{}\n}}",
        binding,
        right,
        indent_block(&body, "    ")
    );
    if prelude.is_empty() {
        Ok(for_code)
    } else {
        Ok(format!("{}\n{}", prelude, for_code))
    }
}

fn transpile_try_stmt(try_stmt: &TryStmt, scope: &mut Scope) -> Result<String> {
    let mut try_scope = scope.clone();
    let try_body = transpile_block_stmt(&try_stmt.block, "            ", &mut try_scope)?;

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("    let __trust_try_result: Result<(), String> = (|| -> Result<(), String> {\n");
    out.push_str(&try_body);
    if !try_body.is_empty() {
        out.push('\n');
    }
    out.push_str("            Ok(())\n");
    out.push_str("    })();\n");

    if let Some(handler) = &try_stmt.handler {
        let catch_name = match &handler.param {
            Some(pat) => match pat {
                Pat::Ident(ident) => ident.id.sym.to_string(),
                _ => "_err".to_string(),
            },
            None => "_err".to_string(),
        };

        let mut catch_scope = scope.clone();
        catch_scope.insert(catch_name.clone(), "String".to_string());
        let catch_body = transpile_block_stmt(&handler.body, "        ", &mut catch_scope)?;
        out.push_str(&format!("    if let Err({}) = __trust_try_result {{\n", catch_name));
        out.push_str(&catch_body);
        if !catch_body.is_empty() {
            out.push('\n');
        }
        out.push_str("    }\n");
    } else {
        out.push_str("    let _ = __trust_try_result;\n");
    }

    if let Some(finalizer) = &try_stmt.finalizer {
        let mut final_scope = scope.clone();
        let final_body = transpile_block_stmt(finalizer, "    ", &mut final_scope)?;
        out.push_str(&final_body);
        if !final_body.is_empty() {
            out.push('\n');
        }
    }

    out.push('}');
    Ok(out)
}

fn transpile_for_head_binding(head: &ForHead, scope: &mut Scope) -> Result<(String, String)> {
    match head {
        ForHead::VarDecl(var_decl) => {
            if let Some(first) = var_decl.decls.first() {
                if let Pat::Ident(ident) = &first.name {
                    let name = ident.id.sym.to_string();
                    if let Some(ann) = ident.type_ann.as_deref() {
                        let ty = transpile_type_annotation(ann);
                        scope.insert(name.clone(), ty);
                    }
                    if let Some(init) = &first.init {
                        let init_expr = transpile_expression(init, scope)?;
                        let prelude = format!("let mut {} = {};", name, init_expr);
                        return Ok((name, prelude));
                    }
                    return Ok((name, String::new()));
                }
            }
            Ok(("_item".to_string(), String::new()))
        }
        ForHead::Pat(pat) => match &**pat {
            Pat::Ident(ident) => {
                let name = ident.id.sym.to_string();
                if let Some(ann) = ident.type_ann.as_deref() {
                    let ty = transpile_type_annotation(ann);
                    scope.insert(name.clone(), ty);
                }
                Ok((name, String::new()))
            }
            _ => Ok(("_item".to_string(), String::new())),
        },
        ForHead::UsingDecl(_) => Ok(("_item".to_string(), String::new())),
    }
}

fn indent_block(block: &str, indent: &str) -> String {
    block
        .lines()
        .map(|line| {
            let mut s = String::with_capacity(indent.len() + line.len());
            s.push_str(indent);
            s.push_str(line);
            s
        })
        .collect::<Vec<_>>()
        .join("\n")
}
