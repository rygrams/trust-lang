use swc_ecma_ast::*;

pub fn transpile_type(ts_type: &TsType) -> String {
    match ts_type {
        TsType::TsKeywordType(keyword) => match keyword.kind {
            TsKeywordTypeKind::TsNumberKeyword => "i32".to_string(),
            TsKeywordTypeKind::TsStringKeyword => "String".to_string(),
            TsKeywordTypeKind::TsBooleanKeyword => "bool".to_string(),
            _ => "()".to_string(),
        },
        TsType::TsTypeRef(type_ref) => {
            if let TsEntityName::Ident(ident) = &type_ref.type_name {
                let type_name = ident.sym.to_string();

                // Resolve type arguments if present
                let type_args: Vec<String> = type_ref
                    .type_params
                    .as_ref()
                    .map(|params| params.params.iter().map(|p| transpile_type(p)).collect())
                    .unwrap_or_default();

                match type_name.as_str() {
                    // Primitive integers
                    "number8" => "i8".to_string(),
                    "number16" => "i16".to_string(),
                    "number32" => "i32".to_string(),
                    "number64" => "i64".to_string(),
                    // Floats
                    "float32" => "f32".to_string(),
                    "float64" => "f64".to_string(),
                    // number fallback
                    "number" => "i32".to_string(),
                    // Pointer<T> → Rc<RefCell<T>>  (shared mutable reference, single-thread)
                    "Pointer" => {
                        let inner = type_args.first().cloned().unwrap_or_else(|| "()".to_string());
                        format!("Rc<RefCell<{}>>", inner)
                    }
                    // Threaded<T> → Arc<Mutex<T>>  (shared mutable reference, multi-thread)
                    "Threaded" => {
                        let inner = type_args.first().cloned().unwrap_or_else(|| "()".to_string());
                        format!("Arc<Mutex<{}>>", inner)
                    }
                    // Map<K, V> → HashMap<K, V>
                    "Map" => {
                        format!("HashMap<{}>", type_args.join(", "))
                    }
                    // Set<T> → HashSet<T>
                    "Set" => {
                        let inner = type_args.first().cloned().unwrap_or_else(|| "()".to_string());
                        format!("HashSet<{}>", inner)
                    }
                    // Pass-through generics: Box<T>, Vec<T>, Rc<T>, Arc<T>, …
                    name if !type_args.is_empty() => {
                        format!("{}<{}>", name, type_args.join(", "))
                    }
                    // Custom types (struct, enum) — pass-through as-is
                    other => other.to_string(),
                }
            } else {
                "i32".to_string()
            }
        }
        // T[] → Vec<T>
        TsType::TsArrayType(arr) => format!("Vec<{}>", transpile_type(&arr.elem_type)),
        _ => "()".to_string(),
    }
}

pub fn transpile_type_annotation(type_ann: &TsTypeAnn) -> String {
    transpile_type(&type_ann.type_ann)
}

#[cfg(test)]
mod tests {
    use super::*;
}
