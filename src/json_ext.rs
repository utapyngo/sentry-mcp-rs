use serde_json::Value;

/// Extension trait for ergonomic JSON value access.
pub trait ValueExt {
    fn str_field(&self, key: &str) -> Option<&str>;
    fn i64_field(&self, key: &str) -> Option<i64>;
    fn bool_field(&self, key: &str) -> Option<bool>;
    fn array_field(&self, key: &str) -> Option<&Vec<Value>>;
    fn object_field(&self, key: &str) -> Option<&serde_json::Map<String, Value>>;
}

impl ValueExt for Value {
    fn str_field(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }
    fn i64_field(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Value::as_i64)
    }
    fn bool_field(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }
    fn array_field(&self, key: &str) -> Option<&Vec<Value>> {
        self.get(key).and_then(Value::as_array)
    }
    fn object_field(&self, key: &str) -> Option<&serde_json::Map<String, Value>> {
        self.get(key).and_then(Value::as_object)
    }
}
