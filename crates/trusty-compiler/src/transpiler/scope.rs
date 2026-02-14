use std::collections::HashMap;

/// Maps variable/parameter names to their Rust type strings within a function.
pub type Scope = HashMap<String, String>;

/// Returns true if the Rust type string represents a Pointer<T> (Rc<RefCell<T>>).
pub fn is_pointer(type_str: &str) -> bool {
    type_str.starts_with("Rc<RefCell<")
}

/// Returns true if the Rust type string represents a Threaded<T> (Arc<Mutex<T>>).
pub fn is_threaded(type_str: &str) -> bool {
    type_str.starts_with("Arc<Mutex<")
}
