//! The pure JSON value model.
//!
//! Objects preserve insertion order (`Vec<(String, Value)>`) so documents
//! round-trip deterministically. Numbers stay lossless: integers never pass
//! through `f64`.

/// A JSON number, kept lossless.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

impl Number {
    pub fn as_f64(&self) -> f64 {
        match self {
            Number::PosInt(v) => *v as f64,
            Number::NegInt(v) => *v as f64,
            Number::Float(v) => *v,
        }
    }

    pub fn is_integer(&self) -> bool {
        !matches!(self, Number::Float(_))
    }
}

/// A JSON value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Human-readable JSON type name, used in [`crate::JsonError::TypeMismatch`].
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<Number> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().map(|n| n.as_f64())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(entries) => Some(entries),
            _ => None,
        }
    }

    /// Object member lookup by key (first match wins; keys are unique in
    /// practice because constructors dedupe).
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Build an object from key/value pairs, replacing duplicate keys.
    pub fn object<I, K, V>(entries: I) -> Value
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<Value>,
    {
        let mut obj: Vec<(String, Value)> = Vec::new();
        for (k, v) in entries {
            let (k, v) = (k.into(), v.into());
            match obj.iter_mut().find(|(ek, _)| *ek == k) {
                Some((_, slot)) => *slot = v,
                None => obj.push((k, v)),
            }
        }
        Value::Object(obj)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Number(Number::PosInt(v))
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        if v >= 0 {
            Value::Number(Number::PosInt(v as u64))
        } else {
            Value::Number(Number::NegInt(v))
        }
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(Number::Float(v))
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names() {
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::from(true).type_name(), "boolean");
        assert_eq!(Value::from(1u64).type_name(), "number");
        assert_eq!(Value::from(1.5f64).type_name(), "number");
        assert_eq!(Value::from("x").type_name(), "string");
        assert_eq!(Value::from(vec![]).type_name(), "array");
        assert_eq!(Value::object(Vec::<(&str, Value)>::new()).type_name(), "object");
    }

    #[test]
    fn object_lookup_and_dedupe() {
        let doc = Value::object([
            ("a", Value::from(1u64)),
            ("b", Value::from("two")),
            ("a", Value::from(3u64)),
        ]);
        assert_eq!(doc.get("a"), Some(&Value::from(3u64)));
        assert_eq!(doc.get("b"), Some(&Value::from("two")));
        assert_eq!(doc.get("missing"), None);
        // duplicate key replaced, order of first occurrence kept
        assert_eq!(
            doc.as_object().unwrap(),
            &[("a".to_string(), Value::from(3u64)), ("b".to_string(), Value::from("two"))]
        );
    }

    #[test]
    fn numbers_stay_lossless() {
        let big = Value::from(u64::MAX);
        assert_eq!(big.as_number(), Some(Number::PosInt(u64::MAX)));
        assert!(big.as_number().unwrap().is_integer());
        let neg = Value::from(-5i64);
        assert_eq!(neg.as_number(), Some(Number::NegInt(-5)));
        assert_eq!(Value::from(2.5f64).as_f64(), Some(2.5));
    }

    #[test]
    fn accessors_are_type_gated() {
        let v = Value::from("s");
        assert_eq!(v.as_str(), Some("s"));
        assert_eq!(v.as_array(), None);
        assert!(Value::Null.is_null());
    }
}
