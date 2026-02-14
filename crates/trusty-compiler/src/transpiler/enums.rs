use anyhow::Result;
use swc_ecma_ast::*;

pub fn transpile_enum(decl: &TsEnumDecl) -> Result<String> {
    let name = &decl.id.sym;
    let mut variants = Vec::new();

    for member in &decl.members {
        let variant_name = match &member.id {
            TsEnumMemberId::Ident(ident) => ident.sym.to_string(),
            TsEnumMemberId::Str(s) => s.value.to_string_lossy().into_owned(),
        };
        // Optional discriminant value (e.g. Red = 0)
        variants.push(format!("    {}", variant_name));
    }

    Ok(format!(
        "#[derive(Debug, Clone)]\nenum {} {{\n{},\n}}",
        name,
        variants.join(",\n")
    ))
}
