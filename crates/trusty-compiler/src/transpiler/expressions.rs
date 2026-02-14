use super::scope::{is_pointer, Scope};
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
        _ => Ok("unknown_expr".to_string()),
    }
}

/// Field access: `p.name` → `p.borrow().name` if p is Pointer<T>
fn transpile_member_access(member: &MemberExpr, scope: &Scope) -> Result<String> {
    let obj_str = transpile_expression(&member.obj, scope)?;
    let prop = match &member.prop {
        MemberProp::Ident(ident) => ident.sym.to_string(),
        _ => "unknown".to_string(),
    };
    if let Some(name) = ident_name(&member.obj) {
        if scope.get(&name).map(|t| is_pointer(t)).unwrap_or(false) {
            return Ok(format!("{}.borrow().{}", obj_str, prop));
        }
    }
    Ok(format!("{}.{}", obj_str, prop))
}

/// Assignment: `p.name = x` → `p.borrow_mut().name = x` if p is Pointer<T>
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
                if scope.get(&name).map(|t| is_pointer(t)).unwrap_or(false) {
                    return Ok(format!("{}.borrow_mut().{} = {}", obj_str, prop, value));
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

    // Uppercase object = Rust type → use `::` (e.g. Instant::now())
    let separator = if obj.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        "::"
    } else {
        "."
    };
    Ok(format!("{}{}{}({})", obj, separator, prop, arg_strs?.join(", ")))
}
