use super::scope::{Scope, MODULE_ALIAS_MARKER};
use super::statements::transpile_block_stmt;
use super::types::*;
use anyhow::{bail, Result};
use swc_ecma_ast::*;

pub fn transpile_function(func: &FnDecl, module_aliases: &[String]) -> Result<String> {
    let name = &func.ident.sym;
    let mut scope = base_scope(module_aliases);
    let params = transpile_params(&func.function.params, &mut scope)?;
    let return_type = transpile_return_type(&func.function.return_type)?;
    if func.function.is_async {
        let body = transpile_async_block(&func.function.body, &mut scope)?;
        return Ok(format!(
            "fn {}({}) -> std::thread::JoinHandle<{}> {{\n    std::thread::spawn(move || {{\n{}\n    }})\n}}",
            name, params, return_type, body
        ));
    }

    let body = transpile_block(&func.function.body, &mut scope)?;
    Ok(format!("fn {}({}) -> {} {{\n{}\n}}", name, params, return_type, body))
}

pub fn transpile_impl_block(class_decl: &ClassDecl, module_aliases: &[String]) -> Result<Option<String>> {
    let name = class_decl.ident.sym.to_string();
    let mut methods = Vec::new();

    for member in &class_decl.class.body {
        if let ClassMember::Method(method) = member {
            if let Some(code) = transpile_impl_method(method, module_aliases)? {
                methods.push(code);
            }
        }
    }

    if methods.is_empty() {
        return Ok(None);
    }

    Ok(Some(format!("impl {} {{\n{}\n}}", name, methods.join("\n\n"))))
}

fn transpile_impl_method(method: &ClassMethod, module_aliases: &[String]) -> Result<Option<String>> {
    if method.is_static {
        return Ok(None);
    }
    if method.function.is_async {
        bail!("`async` methods in `implements` are not supported yet.");
    }

    let name = match &method.key {
        PropName::Ident(ident) => ident.sym.to_string(),
        _ => return Ok(None),
    };

    let mut scope = base_scope(module_aliases);
    let params = transpile_params(&method.function.params, &mut scope)?;
    let return_type = transpile_return_type(&method.function.return_type)?;
    let body = transpile_block(&method.function.body, &mut scope)?;
    let self_param = if method_needs_mut_self(&method.function) {
        "&mut self".to_string()
    } else {
        "&self".to_string()
    };
    let signature_params = if params.is_empty() {
        self_param
    } else {
        format!("{}, {}", self_param, params)
    };

    Ok(Some(format!(
        "    fn {}({}) -> {} {{\n{}\n    }}",
        name, signature_params, return_type, body
    )))
}

fn transpile_params(params: &[Param], scope: &mut Scope) -> Result<String> {
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

            scope.insert(name.clone(), type_str.clone());
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

fn transpile_block(block: &Option<BlockStmt>, scope: &mut Scope) -> Result<String> {
    if let Some(block) = block {
        transpile_block_stmt(block, "    ", scope)
    } else {
        Ok(String::new())
    }
}

fn transpile_async_block(block: &Option<BlockStmt>, scope: &mut Scope) -> Result<String> {
    if let Some(block) = block {
        transpile_block_stmt(block, "        ", scope)
    } else {
        Ok(String::new())
    }
}

fn method_needs_mut_self(function: &Function) -> bool {
    function
        .body
        .as_ref()
        .map(|body| body.stmts.iter().any(stmt_mutates_this))
        .unwrap_or(false)
}

fn stmt_mutates_this(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr_stmt) => expr_mutates_this(&expr_stmt.expr),
        Stmt::Block(block) => block.stmts.iter().any(stmt_mutates_this),
        Stmt::If(if_stmt) => {
            expr_mutates_this(&if_stmt.test)
                || stmt_mutates_this(&if_stmt.cons)
                || if_stmt.alt.as_ref().map(|s| stmt_mutates_this(s)).unwrap_or(false)
        }
        Stmt::Decl(Decl::Var(var_decl)) => var_decl
            .decls
            .iter()
            .filter_map(|d| d.init.as_ref())
            .any(|expr| expr_mutates_this(expr)),
        Stmt::Return(ret) => ret.arg.as_ref().map(|e| expr_mutates_this(e)).unwrap_or(false),
        _ => false,
    }
}

fn expr_mutates_this(expr: &Expr) -> bool {
    match expr {
        Expr::Assign(assign) => {
            assign_target_is_this_member(&assign.left) || expr_mutates_this(&assign.right)
        }
        Expr::Update(update) => expr_is_this_member(&update.arg),
        Expr::Call(call) => {
            call.args.iter().any(|a| expr_mutates_this(&a.expr))
                || matches!(&call.callee, Callee::Expr(callee) if expr_mutates_this(callee))
        }
        Expr::Bin(bin) => expr_mutates_this(&bin.left) || expr_mutates_this(&bin.right),
        Expr::Unary(unary) => expr_mutates_this(&unary.arg),
        Expr::Cond(cond) => {
            expr_mutates_this(&cond.test)
                || expr_mutates_this(&cond.cons)
                || expr_mutates_this(&cond.alt)
        }
        Expr::Paren(paren) => expr_mutates_this(&paren.expr),
        Expr::Seq(seq) => seq.exprs.iter().any(|e| expr_mutates_this(e)),
        _ => false,
    }
}

fn assign_target_is_this_member(target: &AssignTarget) -> bool {
    match target {
        AssignTarget::Simple(SimpleAssignTarget::Member(member)) => this_member(member),
        _ => false,
    }
}

fn expr_is_this_member(expr: &Expr) -> bool {
    match expr {
        Expr::Member(member) => this_member(member),
        _ => false,
    }
}

fn this_member(member: &MemberExpr) -> bool {
    matches!(&*member.obj, Expr::This(_))
}

fn base_scope(module_aliases: &[String]) -> Scope {
    let mut scope = Scope::new();
    for alias in module_aliases {
        scope.insert(alias.clone(), MODULE_ALIAS_MARKER.to_string());
    }
    scope
}
