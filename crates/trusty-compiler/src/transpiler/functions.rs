use super::statements::*;
use super::types::*;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_function(func: &FnDecl) -> Result<String> {
    let name = &func.ident.sym;
    let params = transpile_params(&func.function.params)?;
    let return_type = transpile_return_type(&func.function.return_type)?;
    let body = transpile_block(&func.function.body)?;

    Ok(format!(
        "fn {}({}) -> {} {{\n{}\n}}",
        name, params, return_type, body
    ))
}

fn transpile_params(params: &[Param]) -> Result<String> {
    let param_strs: Vec<String> = params
        .iter()
        .map(|p| {
            let name = match &p.pat {
                Pat::Ident(ident) => ident.id.sym.to_string(),
                _ => "unknown".to_string(),
            };

            let type_str = param_type_annotation(&p.pat)
                .map(transpile_type_annotation)
                .unwrap_or_else(|| "i32".to_string());

            format!("{}: {}", name, type_str)
        })
        .collect();

    Ok(param_strs.join(", "))
}

fn param_type_annotation(pat: &Pat) -> Option<&TsTypeAnn> {
    match pat {
        Pat::Ident(ident) => ident.type_ann.as_deref(),
        Pat::Array(array) => array.type_ann.as_deref(),
        Pat::Object(object) => object.type_ann.as_deref(),
        Pat::Rest(rest) => rest.type_ann.as_deref(),
        _ => None,
    }
}

fn transpile_return_type(return_type: &Option<Box<TsTypeAnn>>) -> Result<String> {
    if let Some(type_ann) = return_type {
        Ok(transpile_type(&type_ann.type_ann))
    } else {
        Ok("()".to_string())
    }
}

fn transpile_block(block: &Option<BlockStmt>) -> Result<String> {
    if let Some(block) = block {
        let mut statements = Vec::new();

        for stmt in &block.stmts {
            let rust_stmt = transpile_statement(stmt)?;
            statements.push(format!("    {}", rust_stmt));
        }

        Ok(statements.join("\n"))
    } else {
        Ok(String::new())
    }
}
