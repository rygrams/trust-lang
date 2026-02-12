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
                match type_name.as_str() {
                    // Entiers
                    "number8" => "i8",
                    "number16" => "i16",
                    "number32" => "i32",
                    "number64" => "i64",
                    // Flottants
                    "float32" => "f32",
                    "float64" => "f64",
                    // Fallback
                    "number" => "i32",
                    _ => "i32",
                }
                .to_string()
            } else {
                "i32".to_string()
            }
        }
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
