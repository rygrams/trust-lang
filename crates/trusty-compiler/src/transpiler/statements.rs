use super::expressions::*;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_statement(stmt: &Stmt) -> Result<String> {
    match stmt {
        Stmt::Return(return_stmt) => {
            if let Some(arg) = &return_stmt.arg {
                let expr = transpile_expression(arg)?;
                Ok(expr)
            } else {
                Ok("()".to_string())
            }
        }
        Stmt::Expr(expr_stmt) => {
            let expr = transpile_expression(&expr_stmt.expr)?;
            Ok(format!("{};", expr))
        }
        _ => Ok("// Statement non supporté".to_string()),
    }
}
