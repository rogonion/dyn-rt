//! Dev-only helpers shared by dyn-rt test suites (ADR 0004).
//!
//! Loads the canonical case tables from `tests/shared/` and converts between
//! `serde_json::Value` and the domain [`Value`] so suites can stay thin loops
//! over cases. This crate must only ever be referenced from
//! `dev-dependencies` — never from shipped code.

use std::path::PathBuf;

use dyn_rt_json_domain::{Number, Value};

fn shared_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../tests/shared")
}

/// Load a case table from `tests/shared/<rel>` (a JSON array of case objects).
pub fn load_cases(rel: &str) -> Vec<serde_json::Value> {
    let path = shared_dir().join(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

/// Build the subject value of a case from its `build` instruction:
/// `{"value": <any json>}` or `{"object": [[key, value], ...]}`.
pub fn build_value(build: &serde_json::Value) -> Value {
    if let Some(v) = build.get("value") {
        return to_value(v);
    }
    if let Some(entries) = build.get("object").and_then(|o| o.as_array()) {
        let pairs = entries.iter().map(|e| {
            let pair = e.as_array().expect("object entry must be [key, value]");
            (
                pair[0].as_str().expect("object key must be a string").to_string(),
                to_value(&pair[1]),
            )
        });
        return Value::object(pairs);
    }
    panic!("build instruction must contain 'value' or 'object': {build}");
}

/// Convert a `serde_json::Value` into the domain [`Value`] (test-side only;
/// the shipped converter belongs to `dyn-rt-json-infra`).
pub fn to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => Value::Number(if let Some(u) = n.as_u64() {
            Number::PosInt(u)
        } else if let Some(i) = n.as_i64() {
            Number::NegInt(i)
        } else {
            Number::Float(n.as_f64().expect("number must be u64, i64, or f64"))
        }),
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => Value::Array(items.iter().map(to_value).collect()),
        serde_json::Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), to_value(v))).collect())
        }
    }
}

/// Convert a domain [`Value`] back into `serde_json::Value` (order-preserving).
pub fn to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(Number::PosInt(u)) => (*u).into(),
        Value::Number(Number::NegInt(i)) => (*i).into(),
        Value::Number(Number::Float(f)) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .expect("finite f64"),
        Value::String(s) => s.clone().into(),
        Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(to_json).collect())
        }
        Value::Object(entries) => {
            let map: serde_json::Map<String, serde_json::Value> =
                entries.iter().map(|(k, v)| (k.clone(), to_json(v))).collect();
            serde_json::Value::Object(map)
        }
    }
}

/// Compact JSON string of a fixture value (order-preserving).
pub fn compact(v: &serde_json::Value) -> String {
    serde_json::to_string(v).expect("fixture must serialize")
}

/// Compact JSON string of a domain value (order-preserving) — string equality
/// on this also verifies object key order.
pub fn compact_value(v: &Value) -> String {
    compact(&to_json(v))
}
