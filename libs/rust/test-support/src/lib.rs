//! Dev-only helpers shared by dyn-rt test suites (ADR 0004).
//!
//! Loads the canonical case tables from `tests/shared/` and converts between
//! `serde_json::Value` and the domain [`Value`] so suites can stay thin loops
//! over cases. This crate must only ever be referenced from
//! `dev-dependencies` — never from shipped code.

use std::path::PathBuf;

use dyn_rt_json_domain::path::{
    AccessStrategy, PathSegment, PathSelector, RecursiveDescentSegment, RecursiveDescentSegments,
    SliceSelector,
};
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
                pair[0]
                    .as_str()
                    .expect("object key must be a string")
                    .to_string(),
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
        Value::Array(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        Value::Object(entries) => {
            let map: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| (k.clone(), to_json(v)))
                .collect();
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

// ---------------------------------------------------------------------------
// Path AST <-> JSON conversions for the shared path vectors
// (tests/shared/json/path-*.json). Canonical segment form:
//   { "access": "mixed"|"named"|"positional",
//     "selector": { "kind": "root"|"wildcard"|"key"|"index"|"slice"|"union", ... } }
// ---------------------------------------------------------------------------

pub fn access_from_json(v: &serde_json::Value) -> AccessStrategy {
    match v.as_str().expect("access must be a string") {
        "mixed" => AccessStrategy::Mixed,
        "named" => AccessStrategy::Named,
        "positional" => AccessStrategy::Positional,
        other => panic!("unknown access strategy: {other}"),
    }
}

pub fn access_to_json(access: AccessStrategy) -> serde_json::Value {
    match access {
        AccessStrategy::Mixed => "mixed".into(),
        AccessStrategy::Named => "named".into(),
        AccessStrategy::Positional => "positional".into(),
    }
}

pub fn segment_from_json(v: &serde_json::Value) -> PathSegment {
    let access = access_from_json(&v["access"]);
    let selector_json = &v["selector"];
    let selector = match selector_json["kind"]
        .as_str()
        .expect("selector.kind must be a string")
    {
        "root" => PathSelector::Root,
        "wildcard" => PathSelector::Wildcard,
        "key" => PathSelector::Key(
            selector_json["value"]
                .as_str()
                .expect("key value must be a string")
                .to_string(),
        ),
        "index" => PathSelector::Index(
            selector_json["value"]
                .as_i64()
                .expect("index value must be an integer") as isize,
        ),
        "slice" => PathSelector::Slice(SliceSelector {
            start: selector_json["start"].as_i64().map(|n| n as isize),
            end: selector_json["end"].as_i64().map(|n| n as isize),
            step: selector_json["step"].as_i64().map(|n| n as isize),
        }),
        "union" => PathSelector::Union(
            selector_json["members"]
                .as_array()
                .expect("union members must be an array")
                .iter()
                .map(segment_from_json)
                .collect(),
        ),
        other => panic!("unknown selector kind: {other}"),
    };
    PathSegment {
        access_strategy: access,
        selector,
    }
}

pub fn segment_to_json(segment: &PathSegment) -> serde_json::Value {
    let selector = match &segment.selector {
        PathSelector::Unrecognized => serde_json::json!({ "kind": "unrecognized" }),
        PathSelector::Root => serde_json::json!({ "kind": "root" }),
        PathSelector::Wildcard => serde_json::json!({ "kind": "wildcard" }),
        PathSelector::Key(key) => serde_json::json!({ "kind": "key", "value": key }),
        PathSelector::Index(index) => serde_json::json!({ "kind": "index", "value": index }),
        PathSelector::Slice(slice) => serde_json::json!({
            "kind": "slice",
            "start": slice.start,
            "end": slice.end,
            "step": slice.step,
        }),
        PathSelector::Union(members) => serde_json::json!({
            "kind": "union",
            "members": members.iter().map(segment_to_json).collect::<Vec<_>>(),
        }),
    };
    serde_json::json!({ "access": access_to_json(segment.access_strategy), "selector": selector })
}

/// Convert the top-level `segments` array of a path-parse case.
pub fn segments_from_json(v: &serde_json::Value) -> RecursiveDescentSegments {
    RecursiveDescentSegments(
        v.as_array()
            .expect("segments must be an array of descent groups")
            .iter()
            .map(|group| {
                RecursiveDescentSegment(
                    group
                        .as_array()
                        .expect("descent group must be an array of segments")
                        .iter()
                        .map(segment_from_json)
                        .collect(),
                )
            })
            .collect(),
    )
}

/// Convert a parsed AST back to the top-level `segments` JSON form.
pub fn segments_to_json(segments: &RecursiveDescentSegments) -> serde_json::Value {
    serde_json::Value::Array(
        segments
            .0
            .iter()
            .map(|group| serde_json::Value::Array(group.0.iter().map(segment_to_json).collect()))
            .collect(),
    )
}
