use anyhow::Result;
use swc_ecma_ast::*;

enum Discriminant {
    Num(i64),
    Str(String),
    None,
}

pub fn transpile_enum(decl: &TsEnumDecl) -> Result<String> {
    let name = &decl.id.sym;
    let mut variants = Vec::new();
    let mut string_values: Vec<(String, String)> = Vec::new();

    for member in &decl.members {
        let variant_name = match &member.id {
            TsEnumMemberId::Ident(ident) => ident.sym.to_string(),
            TsEnumMemberId::Str(s) => s.value.to_string_lossy().into_owned(),
        };

        let discriminant = match &member.init {
            Some(init) => match &**init {
                Expr::Lit(Lit::Num(n)) => Discriminant::Num(n.value as i64),
                Expr::Lit(Lit::Str(s)) => Discriminant::Str(s.value.to_string_lossy().into_owned()),
                _ => Discriminant::None,
            },
            None => Discriminant::None,
        };

        match discriminant {
            Discriminant::Num(v) => variants.push(format!("    {} = {}", variant_name, v)),
            Discriminant::Str(s) => {
                variants.push(format!("    {}", variant_name));
                string_values.push((variant_name, s));
            }
            Discriminant::None => variants.push(format!("    {}", variant_name)),
        }
    }

    let enum_def = format!(
        "#[derive(Debug, Clone)]\nenum {} {{\n{},\n}}",
        name,
        variants.join(",\n")
    );

    if string_values.is_empty() {
        return Ok(enum_def);
    }

    // Generate as_str() and Display impl for string enums
    let as_str_arms: String = string_values
        .iter()
        .map(|(v, s)| format!("            {}::{} => \"{}\"", name, v, s))
        .collect::<Vec<_>>()
        .join(",\n");

    let impl_block = format!(
        "impl {} {{\n    pub fn as_str(&self) -> &'static str {{\n        match self {{\n{}\n        }}\n    }}\n}}\n\nimpl std::fmt::Display for {} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        write!(f, \"{{}}\", self.as_str())\n    }}\n}}",
        name, as_str_arms, name
    );

    Ok(format!("{}\n\n{}", enum_def, impl_block))
}
