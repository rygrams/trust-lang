use super::scope::{is_pointer, is_threaded, Scope};
use super::statements::transpile_block_stmt;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_expression(expr: &Expr, scope: &Scope) -> Result<String> {
    match expr {
        Expr::Bin(bin_expr) => {
            let left = transpile_expression(&bin_expr.left, scope)?;
            let right = transpile_expression(&bin_expr.right, scope)?;
            let op = match bin_expr.op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Lt => "<",
                BinaryOp::LtEq => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::GtEq => ">=",
                BinaryOp::EqEq | BinaryOp::EqEqEq => "==",
                BinaryOp::NotEq | BinaryOp::NotEqEq => "!=",
                BinaryOp::LogicalAnd => "&&",
                BinaryOp::LogicalOr => "||",
                _ => "?",
            };
            Ok(format!("{} {} {}", left, op, right))
        }
        Expr::Ident(ident) => Ok(ident.sym.to_string()),
        Expr::Lit(lit) => match lit {
            Lit::Num(num) => Ok(num.value.to_string()),
            Lit::Str(s) => Ok(format!("\"{}\".to_string()", s.value.to_string_lossy())),
            Lit::Bool(b) => Ok(b.value.to_string()),
            _ => Ok("unknown_literal".to_string()),
        },
        Expr::Tpl(tpl) => transpile_template_literal(tpl, scope),
        Expr::Call(call) => transpile_call_expression(call, scope),
        Expr::Array(array_lit) => {
            let elems: Result<Vec<String>> = array_lit
                .elems
                .iter()
                .filter_map(|e| e.as_ref())
                .map(|e| transpile_expression(&e.expr, scope))
                .collect();
            Ok(format!("vec![{}]", elems?.join(", ")))
        }
        Expr::Member(member) => transpile_member_access(member, scope),
        Expr::Assign(assign) => transpile_assign(assign, scope),
        Expr::Arrow(arrow) => transpile_arrow(arrow, scope),
        Expr::Paren(paren) => transpile_expression(&paren.expr, scope),
        _ => Ok("unknown_expr".to_string()),
    }
}

/// Field access: transparent borrow for Pointer<T> and Threaded<T>
fn transpile_member_access(member: &MemberExpr, scope: &Scope) -> Result<String> {
    let obj_str = transpile_expression(&member.obj, scope)?;

    // arr[i] → arr[i as usize]
    if let MemberProp::Computed(computed) = &member.prop {
        let idx = transpile_expression(&computed.expr, scope)?;
        return Ok(format!("{}[{} as usize]", obj_str, idx));
    }

    let prop = match &member.prop {
        MemberProp::Ident(ident) => ident.sym.to_string(),
        _ => "unknown".to_string(),
    };
    // .length → .len()
    if prop == "length" {
        return Ok(format!("{}.len()", obj_str));
    }

    if let Some(name) = ident_name(&member.obj) {
        if let Some(ty) = scope.get(&name) {
            if is_pointer(ty) {
                return Ok(format!("{}.borrow().{}", obj_str, prop));
            }
            if is_threaded(ty) {
                return Ok(format!("{}.lock().unwrap().{}", obj_str, prop));
            }
        }
    }
    Ok(format!("{}.{}", obj_str, prop))
}

/// Assignment: transparent borrow_mut for Pointer<T> and Threaded<T>
fn transpile_assign(assign: &AssignExpr, scope: &Scope) -> Result<String> {
    let value = transpile_expression(&assign.right, scope)?;
    match &assign.left {
        AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
            let obj_str = transpile_expression(&member.obj, scope)?;
            let prop = match &member.prop {
                MemberProp::Ident(ident) => ident.sym.to_string(),
                _ => "unknown".to_string(),
            };
            if let Some(name) = ident_name(&member.obj) {
                if let Some(ty) = scope.get(&name) {
                    if is_pointer(ty) {
                        return Ok(format!("{}.borrow_mut().{} = {}", obj_str, prop, value));
                    }
                    if is_threaded(ty) {
                        return Ok(format!("{}.lock().unwrap().{} = {}", obj_str, prop, value));
                    }
                }
            }
            Ok(format!("{}.{} = {}", obj_str, prop, value))
        }
        AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) => {
            Ok(format!("{} = {}", ident.id.sym, value))
        }
        _ => Ok("// assignment non supporté".to_string()),
    }
}

/// Arrow function: `() => expr` or `(x) => expr` → `move || expr`
fn transpile_arrow(arrow: &ArrowExpr, scope: &Scope) -> Result<String> {
    let params: Vec<String> = arrow
        .params
        .iter()
        .map(|p| match p {
            Pat::Ident(ident) => ident.id.sym.to_string(),
            _ => "_".to_string(),
        })
        .collect();

    let body = match &*arrow.body {
        BlockStmtOrExpr::Expr(expr) => transpile_expression(expr, scope)?,
        BlockStmtOrExpr::BlockStmt(block) => {
            let mut inner_scope = scope.clone();
            let stmts = transpile_block_stmt(block, "    ", &mut inner_scope)?;
            format!("{{\n{}\n}}", stmts)
        }
    };

    if params.is_empty() {
        Ok(format!("move || {}", body))
    } else {
        Ok(format!("move |{}| {}", params.join(", "), body))
    }
}

fn ident_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(ident) => Some(ident.sym.to_string()),
        _ => None,
    }
}

fn transpile_template_literal(tpl: &Tpl, scope: &Scope) -> Result<String> {
    let mut format_str = String::new();
    let mut args = Vec::new();

    for (i, quasi) in tpl.quasis.iter().enumerate() {
        format_str.push_str(&quasi.raw.to_string());
        if i < tpl.exprs.len() {
            format_str.push_str("{}");
            let expr = transpile_expression(&tpl.exprs[i], scope)?;
            args.push(expr);
        }
    }

    if args.is_empty() {
        Ok(format!("\"{}\" .to_string()", format_str))
    } else {
        Ok(format!("format!(\"{}\", {})", format_str, args.join(", ")))
    }
}

fn transpile_call_expression(call: &CallExpr, scope: &Scope) -> Result<String> {
    match &call.callee {
        Callee::Expr(expr) => match &**expr {
            Expr::Member(member) => transpile_member_call(member, &call.args, scope),
            Expr::Ident(ident) => {
                let func_name = ident.sym.to_string();
                let args: Result<Vec<String>> = call
                    .args
                    .iter()
                    .map(|arg| transpile_expression(&arg.expr, scope))
                    .collect();
                Ok(format!("{}({})", func_name, args?.join(", ")))
            }
            _ => Ok("unknown_call".to_string()),
        },
        _ => Ok("unknown_callee".to_string()),
    }
}

fn transpile_member_call(member: &MemberExpr, args: &[ExprOrSpread], scope: &Scope) -> Result<String> {
    let obj = transpile_expression(&member.obj, scope)?;
    let prop = match &member.prop {
        MemberProp::Ident(ident) => ident.sym.to_string(),
        _ => "unknown".to_string(),
    };

    // Thread.run(fn) → std::thread::spawn(fn)
    if obj == "Thread" && prop == "run" {
        let arg_strs: Result<Vec<String>> = args
            .iter()
            .map(|arg| transpile_expression(&arg.expr, scope))
            .collect();
        return Ok(format!("std::thread::spawn({})", arg_strs?.join(", ")));
    }

    if obj == "console" && prop == "log" {
        let arg_strs: Result<Vec<String>> = args
            .iter()
            .map(|arg| transpile_expression(&arg.expr, scope))
            .collect();
        return Ok(format!("println!(\"{{}}\", {})", arg_strs?.join(", ")));
    }

    let arg_strs: Result<Vec<String>> = args
        .iter()
        .map(|arg| transpile_expression(&arg.expr, scope))
        .collect();
    let arg_strs = arg_strs?;

    // Array methods
    match prop.as_str() {
        "push" => return Ok(format!("{}.push({})", obj, arg_strs.join(", "))),
        "pop" => return Ok(format!("{}.pop()", obj)),
        "len" => return Ok(format!("{}.len()", obj)),
        "map" => return Ok(format!("{}.iter().map({}).collect::<Vec<_>>()", obj, arg_strs.join(", "))),
        "filter" => return Ok(format!("{}.iter().filter({}).collect::<Vec<_>>()", obj, arg_strs.join(", "))),
        "forEach" => return Ok(format!("{}.iter().for_each({})", obj, arg_strs.join(", "))),
        "includes" => return Ok(format!("{}.contains(&{})", obj, arg_strs.join(", "))),
        "join" => return Ok(format!("{}.join({})", obj, arg_strs.join(", "))),
        "reverse" => return Ok(format!("{{ {}.reverse(); {} }}", obj, obj)),
        "indexOf" => return Ok(format!("{}.iter().position(|r| r == &{}).map(|i| i as i32).unwrap_or(-1)", obj, arg_strs.join(", "))),
        _ => {}
    }

    // Uppercase object = Rust type → use `::` (e.g. Instant::now())
    let separator = if obj.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        "::"
    } else {
        "."
    };
    Ok(format!("{}{}{}({})", obj, separator, prop, arg_strs.join(", ")))
}
