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
            Lit::Str(s) => Ok(format!("\"{}\"", s.value.to_string_lossy())),
            Lit::Bool(b) => Ok(b.value.to_string()),
            _ => Ok("unknown_literal".to_string()),
        },
        _ => Ok("unknown_expr".to_string()),
    }
}
