/// `use` statements injected when `import ... from "trusty:json"` is detected.
pub fn use_statements() -> Vec<&'static str> {
    vec![
        "use serde_json::Value;",
        r#"#[allow(non_snake_case)]
pub fn parseToJSON(json: String) -> Value {
    serde_json::from_str(&json).unwrap_or(Value::Null)
}

#[allow(non_snake_case)]
pub fn stringify<T: serde::Serialize>(value: T) -> String {
    serde_json::to_string(&value).unwrap_or("null".to_string())
}

#[allow(non_snake_case)]
pub fn toJSON<T: serde::Serialize>(value: T) -> String {
    stringify(value)
}

#[allow(non_snake_case)]
pub fn fromJSON<T: serde::de::DeserializeOwned>(json: String) -> T {
    serde_json::from_str(&json).unwrap()
}"#,
    ]
}

/// External crates needed.
pub fn required_crates() -> Vec<(&'static str, &'static str)> {
    vec![("serde", "1"), ("serde_derive", "1"), ("serde_json", "1")]
}
