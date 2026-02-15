use super::types::transpile_type_annotation;
use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_interface(decl: &TsInterfaceDecl, json_enabled: bool) -> Result<String> {
    let name = decl.id.sym.to_string();
    let mut fields = Vec::new();

    for member in &decl.body.body {
        if let TsTypeElement::TsPropertySignature(prop) = member {
            let field_name = match &*prop.key {
                Expr::Ident(ident) => ident.sym.to_string(),
                _ => continue,
            };
            let field_type = prop
                .type_ann
                .as_deref()
                .map(|ann| transpile_type_annotation(ann))
                .unwrap_or_else(|| "i32".to_string());

            // Recursive field: wrap in Box to avoid infinite-size type
            let field_type = if field_type == name {
                format!("Box<{}>", field_type)
            } else {
                field_type
            };

            fields.push(format!("    {}: {}", field_name, field_type));
        }
    }

    let derives = if json_enabled {
        "#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]"
    } else {
        "#[derive(Debug, Clone)]"
    };

    Ok(format!("{}\nstruct {} {{\n{},\n}}", derives, name, fields.join(",\n")))
}
