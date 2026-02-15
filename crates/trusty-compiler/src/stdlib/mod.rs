pub mod time;

pub struct StdlibModule {
    pub use_statements: Vec<String>,
    /// (crate_name, version) pairs for Cargo.toml
    pub required_crates: Vec<(String, String)>,
}

/// Resolves a `trusty:*` module name to its stdlib definition.
/// Returns `None` if the module is not a known stdlib module.
pub fn resolve(module_name: &str) -> Option<StdlibModule> {
    match module_name {
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
        _ => None,
    }
}
