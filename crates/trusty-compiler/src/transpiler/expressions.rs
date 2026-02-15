use super::scope::{is_module_alias_binding, is_pointer, is_threaded, Scope};
use super::statements::transpile_block_stmt;
use crate::stdlib::time as stdlib_time;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_expression(expr: &Expr, scope: &Scope) -> Result<String> {
    match expr {
        Expr::Bin(bin_expr) => {
            let left = transpile_expression(&bin_expr.left, scope)?;
            let right = transpile_expression(&bin_expr.right, scope)?;
            match bin_expr.op {
                BinaryOp::Add => Ok(format!("{} + {}", left, right)),
                BinaryOp::Sub => Ok(format!("{} - {}", left, right)),
                BinaryOp::Mul => Ok(format!("{} * {}", left, right)),
                BinaryOp::Div => Ok(format!("{} / {}", left, right)),
                BinaryOp::Mod => Ok(format!("{} % {}", left, right)),
                BinaryOp::Lt => Ok(format!("{} < {}", left, right)),
                BinaryOp::LtEq => Ok(format!("{} <= {}", left, right)),
                BinaryOp::Gt => Ok(format!("{} > {}", left, right)),
                BinaryOp::GtEq => Ok(format!("{} >= {}", left, right)),
                BinaryOp::EqEq | BinaryOp::EqEqEq => Ok(format!("{} == {}", left, right)),
                BinaryOp::NotEq | BinaryOp::NotEqEq => Ok(format!("{} != {}", left, right)),
                BinaryOp::LogicalAnd => Ok(format!("{} && {}", left, right)),
                BinaryOp::LogicalOr => Ok(format!("{} || {}", left, right)),
                BinaryOp::Exp => transpile_exponentiation(&bin_expr.left, &bin_expr.right, &left, &right, scope),
                _ => Ok("?".to_string()),
            }
        }
        Expr::Ident(ident) => Ok(ident.sym.to_string()),
        Expr::This(_) => Ok("self".to_string()),
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
        Expr::Cond(cond) => {
            let test = transpile_expression(&cond.test, scope)?;
            let cons = transpile_expression(&cond.cons, scope)?;
            let alt = transpile_expression(&cond.alt, scope)?;
            Ok(format!("if {} {{ {} }} else {{ {} }}", test, cons, alt))
        }
        Expr::Member(member) => transpile_member_access(member, scope),
        Expr::Assign(assign) => transpile_assign(assign, scope),
        Expr::Arrow(arrow) => transpile_arrow(arrow, scope),
        Expr::Await(await_expr) => {
            let awaited = transpile_expression(&await_expr.arg, scope)?;
            Ok(format!("({}).join().unwrap()", awaited))
        }
        Expr::Paren(paren) => transpile_expression(&paren.expr, scope),
        Expr::New(new_expr) => {
            if let Expr::Ident(ident) = &*new_expr.callee {
                match ident.sym.as_ref() {
                    "Map" => return Ok("HashMap::new()".to_string()),
                    "Set" => return Ok("HashSet::new()".to_string()),
                    _ => {}
                }
            }
            Ok("unknown_new".to_string())
        }
        _ => Ok("unknown_expr".to_string()),
    }
}

fn transpile_exponentiation(
    left_expr: &Expr,
    _right_expr: &Expr,
    left: &str,
    right: &str,
    scope: &Scope,
) -> Result<String> {
    let left_ty = infer_rust_type(left_expr, scope);
    let out = match left_ty.as_deref() {
        Some("f32") => format!("({} as f32).powf({} as f32)", left, right),
        Some("f64") => format!("({} as f64).powf({} as f64)", left, right),
        Some("i8") => format!("({} as i8).pow(({}).max(0) as u32)", left, right),
        Some("i16") => format!("({} as i16).pow(({}).max(0) as u32)", left, right),
        Some("i32") => format!("({} as i32).pow(({}).max(0) as u32)", left, right),
        Some("i64") => format!("({} as i64).pow(({}).max(0) as u32)", left, right),
        Some("u8") => format!("({} as u8).pow(({}).max(0) as u32)", left, right),
        Some("u16") => format!("({} as u16).pow(({}).max(0) as u32)", left, right),
        Some("u32") => format!("({} as u32).pow(({}).max(0) as u32)", left, right),
        Some("u64") => format!("({} as u64).pow(({}).max(0) as u32)", left, right),
        Some("usize") => format!("({} as usize).pow(({}).max(0) as u32)", left, right),
        Some("isize") => format!("({} as isize).pow(({}).max(0) as u32)", left, right),
        _ => format!("({} as f64).powf({} as f64)", left, right),
    };
    Ok(out)
}

fn infer_rust_type(expr: &Expr, scope: &Scope) -> Option<String> {
    match expr {
        Expr::Ident(ident) => scope.get(&ident.sym.to_string()).cloned(),
        Expr::Lit(Lit::Num(n)) => {
            if n.value.fract() == 0.0 {
                Some("i32".to_string())
            } else {
                Some("f64".to_string())
            }
        }
        Expr::Paren(paren) => infer_rust_type(&paren.expr, scope),
        Expr::Call(call) => match &call.callee {
            Callee::Expr(callee) => match &**callee {
                Expr::Ident(ident) => match ident.sym.as_ref() {
                    "int8" => Some("i8".to_string()),
                    "int16" => Some("i16".to_string()),
                    "int32" | "int" | "number" | "number32" => Some("i32".to_string()),
                    "int64" | "number64" => Some("i64".to_string()),
                    "float32" => Some("f32".to_string()),
                    "float64" | "float" => Some("f64".to_string()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        },
        _ => None,
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
    let module_alias_obj = ident_name(&member.obj)
        .and_then(|n| scope.get(&n).map(|t| is_module_alias_binding(t)))
        .unwrap_or(false);
    if module_alias_obj {
        return Ok(format!("{}::{}", obj_str, prop));
    }
    // .length
    if prop == "length" {
        if let Some(name) = ident_name(&member.obj) {
            if let Some(ty) = scope.get(&name) {
                if is_pointer(ty) {
                    if ty == "Rc<RefCell<String>>" {
                        return Ok(format!("{}.borrow().chars().count() as i32", obj_str));
                    }
                    return Ok(format!("{}.borrow().len()", obj_str));
                }
                if is_threaded(ty) {
                    if ty == "Arc<Mutex<String>>" {
                        return Ok(format!("{}.lock().unwrap().chars().count() as i32", obj_str));
                    }
                    return Ok(format!("{}.lock().unwrap().len()", obj_str));
                }
                if ty == "String" {
                    return Ok(format!("{}.chars().count() as i32", obj_str));
                }
            }
        }
        match &*member.obj {
            Expr::Lit(Lit::Str(_)) | Expr::Tpl(_) => {
                return Ok(format!("{}.chars().count() as i32", obj_str));
            }
            _ => {}
        }
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
                if let Some(ctor_expr) = transpile_struct_constructor_call(&func_name, &call.args, scope)? {
                    return Ok(ctor_expr);
                }
                if let Some(cast_expr) = transpile_builtin_cast_call(&func_name, &call.args, scope)? {
                    return Ok(cast_expr);
                }
                let args: Result<Vec<String>> = call
                    .args
                    .iter()
                    .map(|arg| transpile_expression(&arg.expr, scope))
                    .collect();
                let args = args?;
                if func_name == "log" && args.len() == 2 {
                    return Ok(format!("log_base({}, {})", args[0], args[1]));
                }
                Ok(format!("{}({})", func_name, args.join(", ")))
            }
            _ => Ok("unknown_call".to_string()),
        },
        _ => Ok("unknown_callee".to_string()),
    }
}

fn transpile_struct_constructor_call(func_name: &str, args: &[ExprOrSpread], scope: &Scope) -> Result<Option<String>> {
    let Some(first) = func_name.chars().next() else {
        return Ok(None);
    };
    if !first.is_uppercase() || args.len() != 1 {
        return Ok(None);
    }

    let Expr::Object(obj) = &*args[0].expr else {
        return Ok(None);
    };

    let mut fields = Vec::new();
    for prop in &obj.props {
        let PropOrSpread::Prop(prop) = prop else {
            return Ok(None);
        };
        match &**prop {
            Prop::KeyValue(kv) => {
                let key = match &kv.key {
                    PropName::Ident(id) => id.sym.to_string(),
                    PropName::Str(s) => s.value.to_string_lossy().to_string(),
                    PropName::Num(n) => n.value.to_string(),
                    _ => return Ok(None),
                };
                let val = transpile_expression(&kv.value, scope)?;
                fields.push(format!("{}: {}", key, val));
            }
            Prop::Shorthand(id) => {
                let key = id.sym.to_string();
                fields.push(format!("{}: {}", key, key));
            }
            _ => return Ok(None),
        }
    }

    Ok(Some(format!("{} {{ {} }}", func_name, fields.join(", "))))
}

fn transpile_builtin_cast_call(func_name: &str, args: &[ExprOrSpread], scope: &Scope) -> Result<Option<String>> {
    if args.len() != 1 {
        return Ok(None);
    }

    let arg_expr = &args[0].expr;
    let arg_rendered = transpile_expression(arg_expr, scope)?;
    let arg_type = match &**arg_expr {
        Expr::Ident(ident) => scope.get(&ident.sym.to_string()).cloned(),
        _ => None,
    };

    if func_name == "string" {
        let out = match arg_type.as_deref() {
            Some("Rc<RefCell<String>>") => format!("{}.borrow().to_string()", arg_rendered),
            Some("Arc<Mutex<String>>") => format!("{}.lock().unwrap().to_string()", arg_rendered),
            Some(t) if t.starts_with("Rc<RefCell<") => format!("(*{}.borrow()).to_string()", arg_rendered),
            Some(t) if t.starts_with("Arc<Mutex<") => format!("(*{}.lock().unwrap()).to_string()", arg_rendered),
            _ => format!("({}).to_string()", arg_rendered),
        };
        return Ok(Some(out));
    }

    if func_name == "boolean" {
        let value_expr = match arg_type.as_deref() {
            Some("Rc<RefCell<String>>") => format!("{}.borrow()", arg_rendered),
            Some("Arc<Mutex<String>>") => format!("{}.lock().unwrap()", arg_rendered),
            Some(t) if t.starts_with("Rc<RefCell<") => format!("*{}.borrow()", arg_rendered),
            Some(t) if t.starts_with("Arc<Mutex<") => format!("*{}.lock().unwrap()", arg_rendered),
            _ => arg_rendered.clone(),
        };

        let out = match arg_type.as_deref() {
            Some("bool") | Some("Rc<RefCell<bool>>") | Some("Arc<Mutex<bool>>") => value_expr,
            Some("String" | "Rc<RefCell<String>>" | "Arc<Mutex<String>>") => {
                format!("!({}).is_empty()", value_expr)
            }
            Some(t) if is_numeric_rust_type(t) => format!("({}) != 0", value_expr),
            Some(t) if t.starts_with("Rc<RefCell<") || t.starts_with("Arc<Mutex<") => {
                format!("({}) != 0", value_expr)
            }
            _ => match &**arg_expr {
                Expr::Lit(Lit::Bool(_)) => value_expr,
                Expr::Lit(Lit::Str(_)) | Expr::Tpl(_) => format!("!({}).is_empty()", value_expr),
                Expr::Lit(Lit::Num(_)) => format!("({}) != 0", value_expr),
                expr if is_boolean_like_expr(expr) => value_expr,
                _ => format!("({}) != 0", value_expr),
            },
        };
        return Ok(Some(out));
    }

    let rust_num = match func_name {
        "int8" => Some("i8"),
        "int16" => Some("i16"),
        "int32" => Some("i32"),
        "int64" => Some("i64"),
        "int" => Some("i32"),
        "number8" => Some("i8"),
        "number16" => Some("i16"),
        "number32" => Some("i32"),
        "number64" => Some("i64"),
        "float32" => Some("f32"),
        "float64" => Some("f64"),
        "float" => Some("f64"),
        "number" => Some("i32"),
        _ => None,
    };
    let Some(rust_num) = rust_num else {
        return Ok(None);
    };

    let value_expr = match arg_type.as_deref() {
        Some("Rc<RefCell<String>>") => format!("{}.borrow()", arg_rendered),
        Some("Arc<Mutex<String>>") => format!("{}.lock().unwrap()", arg_rendered),
        Some(t) if t.starts_with("Rc<RefCell<") => format!("*{}.borrow()", arg_rendered),
        Some(t) if t.starts_with("Arc<Mutex<") => format!("*{}.lock().unwrap()", arg_rendered),
        _ => arg_rendered.clone(),
    };

    let string_like = matches!(arg_type.as_deref(), Some("String" | "Rc<RefCell<String>>" | "Arc<Mutex<String>>"))
        || matches!(&**arg_expr, Expr::Lit(Lit::Str(_)) | Expr::Tpl(_));

    let out = if string_like {
        format!("({}).parse::<{}>().unwrap_or_default()", value_expr, rust_num)
    } else {
        format!("({}) as {}", value_expr, rust_num)
    };
    Ok(Some(out))
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

    if obj == "console" && prop == "write" {
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
    let member_type = ident_name(&member.obj).and_then(|n| scope.get(&n).cloned());
    let string_obj = match member_type.as_deref() {
        Some("Rc<RefCell<String>>") => format!("{}.borrow()", obj),
        Some("Arc<Mutex<String>>") => format!("{}.lock().unwrap()", obj),
        _ => obj.clone(),
    };
    let is_string = match &*member.obj {
        Expr::Lit(Lit::Str(_)) | Expr::Tpl(_) => true,
        Expr::Ident(ident) => scope
            .get(&ident.sym.to_string())
            .map(|t| t == "String" || t == "Rc<RefCell<String>>" || t == "Arc<Mutex<String>>")
            .unwrap_or(false),
        _ => false,
    };

    // Map methods
    match prop.as_str() {
        "set" if arg_strs.len() == 2 => return Ok(format!("{}.insert({}, {})", obj, arg_strs[0], arg_strs[1])),
        "get" => return Ok(format!("{}.get(&{})", obj, arg_strs.join(", "))),
        "has" if arg_strs.len() == 1 => {
            let is_set = ident_name(&member.obj)
                .and_then(|n| scope.get(&n))
                .map(|t| t.starts_with("HashSet"))
                .unwrap_or(false);
            if is_set {
                return Ok(format!("{}.contains(&{})", obj, arg_strs[0]));
            }
            return Ok(format!("{}.contains_key(&{})", obj, arg_strs[0]));
        }
        "delete" => return Ok(format!("{}.remove(&{})", obj, arg_strs.join(", "))),
        // Set methods
        "add" => return Ok(format!("{}.insert({})", obj, arg_strs.join(", "))),
        _ => {}
    }

    // String methods
    match prop.as_str() {
        "toUpperCase" => return Ok(format!("{}.to_uppercase()", string_obj)),
        "toLowerCase" => return Ok(format!("{}.to_lowercase()", string_obj)),
        "startsWith" if arg_strs.len() == 1 => return Ok(format!("{}.starts_with(({}).as_str())", string_obj, arg_strs[0])),
        "endsWith" if arg_strs.len() == 1 => return Ok(format!("{}.ends_with(({}).as_str())", string_obj, arg_strs[0])),
        "includes" if is_string && arg_strs.len() == 1 => return Ok(format!("{}.contains(({}).as_str())", string_obj, arg_strs[0])),
        "indexOf" if is_string && arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_s = &{}; match __trust_s.find(({}).as_str()) {{ Some(__trust_byte) => __trust_s.char_indices().take_while(|(i, _)| *i < __trust_byte).count() as i32, None => -1 }} }}",
                string_obj, arg_strs[0]
            ));
        }
        "lastIndexOf" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_s = &{}; match __trust_s.rfind(({}).as_str()) {{ Some(__trust_byte) => __trust_s.char_indices().take_while(|(i, _)| *i < __trust_byte).count() as i32, None => -1 }} }}",
                string_obj, arg_strs[0]
            ));
        }
        "replace" if arg_strs.len() == 2 => {
            return Ok(format!("{}.replacen(({}).as_str(), ({}).as_str(), 1)", string_obj, arg_strs[0], arg_strs[1]));
        }
        "replaceAll" if arg_strs.len() == 2 => {
            return Ok(format!("{}.replace(({}).as_str(), ({}).as_str())", string_obj, arg_strs[0], arg_strs[1]));
        }
        "trim" => return Ok(format!("{}.trim().to_string()", string_obj)),
        "trimStart" => return Ok(format!("{}.trim_start().to_string()", string_obj)),
        "trimEnd" => return Ok(format!("{}.trim_end().to_string()", string_obj)),
        "repeat" if arg_strs.len() == 1 => return Ok(format!("{}.repeat(({}).max(0) as usize)", string_obj, arg_strs[0])),
        "charAt" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_i = ({}) as isize; if __trust_i < 0 {{ String::new() }} else {{ {}.chars().nth(__trust_i as usize).map(|c| c.to_string()).unwrap_or_default() }} }}",
                arg_strs[0], string_obj
            ));
        }
        "at" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len() as isize; let __trust_i = ({}) as isize; let __trust_pos = if __trust_i < 0 {{ __trust_len + __trust_i }} else {{ __trust_i }}; if __trust_pos < 0 || __trust_pos >= __trust_len {{ String::new() }} else {{ __trust_chars[__trust_pos as usize].to_string() }} }}",
                string_obj, arg_strs[0]
            ));
        }
        "split" if arg_strs.is_empty() => return Ok(format!("vec![({}).to_string()]", string_obj)),
        "split" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{}.split(({}).as_str()).map(|s| s.to_string()).collect::<Vec<String>>()",
                string_obj, arg_strs[0]
            ));
        }
        "slice" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len() as isize; let __trust_start = ({}) as isize; let __trust_from = if __trust_start < 0 {{ (__trust_len + __trust_start).max(0) }} else {{ __trust_start.min(__trust_len) }} as usize; __trust_chars[__trust_from..].iter().collect::<String>() }}",
                string_obj, arg_strs[0]
            ));
        }
        "slice" if arg_strs.len() == 2 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len() as isize; let __trust_start = ({}) as isize; let __trust_end = ({}) as isize; let __trust_from = if __trust_start < 0 {{ (__trust_len + __trust_start).max(0) }} else {{ __trust_start.min(__trust_len) }} as usize; let __trust_to = if __trust_end < 0 {{ (__trust_len + __trust_end).max(0) }} else {{ __trust_end.min(__trust_len) }} as usize; if __trust_to <= __trust_from {{ String::new() }} else {{ __trust_chars[__trust_from..__trust_to].iter().collect::<String>() }} }}",
                string_obj, arg_strs[0], arg_strs[1]
            ));
        }
        "substring" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len(); let __trust_start = ({}).max(0) as usize; __trust_chars[__trust_start.min(__trust_len)..].iter().collect::<String>() }}",
                string_obj, arg_strs[0]
            ));
        }
        "substring" if arg_strs.len() == 2 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len(); let __trust_start = ({}).max(0) as usize; let __trust_end = ({}).max(0) as usize; let (__trust_from, __trust_to) = if __trust_start <= __trust_end {{ (__trust_start, __trust_end) }} else {{ (__trust_end, __trust_start) }}; __trust_chars[__trust_from.min(__trust_len)..__trust_to.min(__trust_len)].iter().collect::<String>() }}",
                string_obj, arg_strs[0], arg_strs[1]
            ));
        }
        "substr" if arg_strs.len() == 1 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len() as isize; let __trust_start = ({}) as isize; let __trust_from = if __trust_start < 0 {{ (__trust_len + __trust_start).max(0) }} else {{ __trust_start.min(__trust_len) }} as usize; __trust_chars[__trust_from..].iter().collect::<String>() }}",
                string_obj, arg_strs[0]
            ));
        }
        "substr" if arg_strs.len() == 2 => {
            return Ok(format!(
                "{{ let __trust_chars: Vec<char> = {}.chars().collect(); let __trust_len = __trust_chars.len() as isize; let __trust_start = ({}) as isize; let __trust_count = ({}).max(0) as usize; let __trust_from = if __trust_start < 0 {{ (__trust_len + __trust_start).max(0) }} else {{ __trust_start.min(__trust_len) }} as usize; let __trust_to = (__trust_from + __trust_count).min(__trust_len as usize); __trust_chars[__trust_from..__trust_to].iter().collect::<String>() }}",
                string_obj, arg_strs[0], arg_strs[1]
            ));
        }
        "concat" => {
            let mut fmt = String::from("{}");
            for _ in &arg_strs {
                fmt.push_str("{}");
            }
            let mut fmt_args = vec![format!("({}).to_string()", string_obj)];
            fmt_args.extend(arg_strs.iter().cloned());
            return Ok(format!("format!(\"{}\", {})", fmt, fmt_args.join(", ")));
        }
        _ => {}
    }

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

    // ── trusty:time — Duration static constructors ────────────────────────────
    if obj == "Duration" && arg_strs.len() == 1 {
        if let Some(mapped) = stdlib_time::map_duration_constructor(&prop, &arg_strs[0]) {
            return Ok(mapped);
        }
    }

    // ── trusty:time — duration / instant instance methods ────────────────────
    if let Some(rust_method) = stdlib_time::map_instance_method(&prop) {
        return Ok(format!("{}.{}()", obj, rust_method));
    }

    let module_alias_obj = ident_name(&member.obj)
        .and_then(|n| scope.get(&n).map(|t| is_module_alias_binding(t)))
        .unwrap_or(false);
    if module_alias_obj && prop == "log" && arg_strs.len() == 2 {
        return Ok(format!("{}::log_base({}, {})", obj, arg_strs[0], arg_strs[1]));
    }

    // Uppercase identifier object = Rust type → use `::` (e.g. Instant::now(), Server::http())
    // But only for direct identifier references, not chained calls (e.g. foo().unwrap() uses `.`)
    let separator = if module_alias_obj {
        "::"
    } else {
        match &*member.obj {
            Expr::Ident(ident) if ident.sym.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) => "::",
            _ => ".",
        }
    };
    Ok(format!("{}{}{}({})", obj, separator, prop, arg_strs.join(", ")))
}

fn is_numeric_rust_type(ty: &str) -> bool {
    matches!(ty, "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" | "f32" | "f64")
}

fn is_boolean_like_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(Lit::Bool(_)) => true,
        Expr::Unary(unary) => matches!(unary.op, UnaryOp::Bang),
        Expr::Bin(bin) => matches!(
            bin.op,
            BinaryOp::EqEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEq
                | BinaryOp::NotEqEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::LogicalAnd
                | BinaryOp::LogicalOr
        ),
        Expr::Paren(p) => is_boolean_like_expr(&p.expr),
        Expr::Call(call) => match &call.callee {
            Callee::Expr(callee_expr) => match &**callee_expr {
                Expr::Ident(ident) => ident.sym == "boolean",
                Expr::Member(member) => match &member.prop {
                    MemberProp::Ident(ident) => {
                        matches!(ident.sym.as_ref(), "includes" | "startsWith" | "endsWith" | "has")
                    }
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}
