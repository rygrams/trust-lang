use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_expression(expr: &Expr) -> Result<String> {
    match expr {
        Expr::Bin(bin_expr) => {
            let left = transpile_expression(&bin_expr.left)?;
            let right = transpile_expression(&bin_expr.right)?;
            let op = match bin_expr.op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
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
        Expr::Tpl(tpl) => transpile_template_literal(tpl),
        Expr::Call(call) => transpile_call_expression(call),
        _ => Ok("unknown_expr".to_string()),
    }
}

fn transpile_template_literal(tpl: &Tpl) -> Result<String> {
    let mut format_str = String::new();
    let mut args = Vec::new();

    for (i, quasi) in tpl.quasis.iter().enumerate() {
        format_str.push_str(&quasi.raw.to_string());

        if i < tpl.exprs.len() {
            format_str.push_str("{}");
            let expr = transpile_expression(&tpl.exprs[i])?;
            args.push(expr);
        }
    }

    if args.is_empty() {
        Ok(format!("\"{}\" .to_string()", format_str))
    } else {
        Ok(format!("format!(\"{}\", {})", format_str, args.join(", ")))
    }
}

fn transpile_call_expression(call: &CallExpr) -> Result<String> {
    match &call.callee {
        Callee::Expr(expr) => match &**expr {
            Expr::Member(member) => transpile_member_call(member, &call.args),
            Expr::Ident(ident) => {
                let func_name = ident.sym.to_string();
                let args: Result<Vec<String>> = call
                    .args
                    .iter()
                    .map(|arg| transpile_expression(&arg.expr))
                    .collect();
                Ok(format!("{}({})", func_name, args?.join(", ")))
            }
            _ => Ok("unknown_call".to_string()),
        },
        _ => Ok("unknown_callee".to_string()),
    }
}

fn transpile_member_call(member: &MemberExpr, args: &[ExprOrSpread]) -> Result<String> {
    let obj = transpile_expression(&member.obj)?;

    let prop = match &member.prop {
        MemberProp::Ident(ident) => ident.sym.to_string(),
        _ => "unknown".to_string(),
    };

    // Handle console.log specially
    if obj == "console" && prop == "log" {
        let arg_strs: Result<Vec<String>> = args
            .iter()
            .map(|arg| transpile_expression(&arg.expr))
            .collect();
        return Ok(format!("println!(\"{{}}\", {})", arg_strs?.join(", ")));
    }

    // Generic method call
    let arg_strs: Result<Vec<String>> = args
        .iter()
        .map(|arg| transpile_expression(&arg.expr))
        .collect();
    Ok(format!("{}.{}({})", obj, prop, arg_strs?.join(", ")))
}
