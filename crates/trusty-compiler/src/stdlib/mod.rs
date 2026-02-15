pub mod time;
pub mod math;
pub mod rand;
pub mod json;

pub struct StdlibModule {
    pub use_statements: Vec<String>,
    /// (crate_name, version) pairs for Cargo.toml
    pub required_crates: Vec<(String, String)>,
}

/// Resolves a `trusty:*` module name to its stdlib definition.
/// Returns `None` if the module is not a known stdlib module.
pub fn resolve(module_name: &str) -> Option<StdlibModule> {
    match module_name {
        "math" => Some(StdlibModule {
            use_statements: math::use_statements()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            required_crates: math::required_crates()
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        }),
        "rand" => Some(StdlibModule {
            use_statements: rand::use_statements()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            required_crates: rand::required_crates()
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        }),
        "time" => Some(StdlibModule {
            use_statements: time::use_statements()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            required_crates: time::required_crates()
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        }),
        "json" => Some(StdlibModule {
            use_statements: json::use_statements()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            required_crates: json::required_crates()
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        }),
        _ => None,
    }
}
