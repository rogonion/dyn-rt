//! Data-driven suites for the reflect module, exercised through the facade on
//! domain Values. Vectors: tests/shared/reflect/*.json (ADR 0004).

use dyn_rt::json::Value;
use dyn_rt::reflect::{self, Reflect, ReflectError, ReflectRefMut};
use dyn_rt_test_support as support;

fn ctx(case: &serde_json::Value, name: &str) -> String {
    format!(
        "case '{name}' failed:\n{}",
        serde_json::to_string_pretty(case).unwrap()
    )
}

fn error_kind(err: &ReflectError) -> &'static str {
    match err {
        ReflectError::UnsupportedOperation { .. } => "UnsupportedOperation",
        ReflectError::TypeMismatch { .. } => "TypeMismatch",
        ReflectError::IndexOutOfBounds { .. } => "IndexOutOfBounds",
    }
}

#[test]
fn reflect_kind() {
    for case in support::load_cases("reflect/kind.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let value = support::to_value(&case["value"]);
        let reflected: &dyn Reflect = &value;

        assert_eq!(
            reflect::kind_of(reflected).to_string(),
            case["expect_kind"].as_str().unwrap(),
            "{}",
            ctx(&case, name)
        );
        assert_eq!(
            reflected.reflect_type_name(),
            case["expect_type"].as_str().unwrap(),
            "{}",
            ctx(&case, name)
        );
    }
}

#[test]
fn reflect_map_ops() {
    for case in support::load_cases("reflect/map-ops.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let mut value = support::to_value(&case["value"]);
        let op = case["op"].as_str().unwrap();
        let ctx = || ctx(&case, name);

        match op {
            "get" => {
                let key = case["key"].as_str().unwrap();
                let map = reflect::as_map(&value as &dyn Reflect);
                let result = map.and_then(|m| m.reflect_get(key));
                let expected = support::to_value(&case["expected"]);
                match (&case["expected"], result) {
                    (serde_json::Value::Null, None) => {}
                    _ => assert_eq!(result.map(support::value_of), Some(&expected), "{}", ctx()),
                }
            }
            "contains" => {
                let map = reflect::as_map(&value as &dyn Reflect).expect("map expected");
                assert_eq!(
                    map.reflect_contains_key(case["key"].as_str().unwrap()),
                    case["expected"].as_bool().unwrap(),
                    "{}",
                    ctx()
                );
            }
            "set" => {
                let key = case["key"].as_str().unwrap();
                let new_value: Value = support::to_value(&case["set"]);
                let type_name = value.reflect_type_name().to_string();
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Map(m) => m.reflect_set(key, Box::new(new_value)),
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: "reflect_set",
                        type_name,
                    }),
                };
                match case.get("expect_error") {
                    Some(expected_error) => assert_eq!(
                        error_kind(&result.unwrap_err()),
                        expected_error.as_str().unwrap(),
                        "{}",
                        ctx()
                    ),
                    None => {
                        result.unwrap();
                        assert_eq!(value, support::to_value(&case["expected"]), "{}", ctx());
                    }
                }
            }
            "delete" => {
                let key = case["key"].as_str().unwrap();
                if let Some(map) = reflect::as_map_mut(&mut value as &mut dyn Reflect) {
                    map.reflect_delete(key);
                }
                assert_eq!(value, support::to_value(&case["expected"]), "{}", ctx());
            }
            "keys" => {
                let map = reflect::as_map(&value as &dyn Reflect).expect("map expected");
                let keys = map.reflect_keys();
                let expected: Vec<String> = case["expected"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| k.as_str().unwrap().to_string())
                    .collect();
                assert_eq!(keys, expected, "{}", ctx());
            }
            "len" => {
                let map = reflect::as_map(&value as &dyn Reflect).expect("map expected");
                assert_eq!(
                    map.reflect_len(),
                    case["expected"].as_u64().unwrap() as usize,
                    "{}",
                    ctx()
                );
            }
            other => panic!("unknown map op '{other}' in case '{name}'"),
        }
    }
}

#[test]
fn reflect_sequence_ops() {
    for case in support::load_cases("reflect/sequence-ops.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let mut value = support::to_value(&case["value"]);
        let op = case["op"].as_str().unwrap();
        let ctx = || ctx(&case, name);

        match op {
            "get" => {
                let index = case["index"].as_u64().unwrap() as usize;
                let seq = reflect::as_sequence(&value as &dyn Reflect);
                let result = seq.and_then(|s| s.reflect_get(index));
                let expected = support::to_value(&case["expected"]);
                match (&case["expected"], result) {
                    (serde_json::Value::Null, None) => {}
                    _ => assert_eq!(result.map(support::value_of), Some(&expected), "{}", ctx()),
                }
            }
            "len" => {
                let seq = reflect::as_sequence(&value as &dyn Reflect).expect("sequence expected");
                assert_eq!(
                    seq.reflect_len(),
                    case["expected"].as_u64().unwrap() as usize,
                    "{}",
                    ctx()
                );
            }
            "set" | "push" => {
                let new_value: Value = support::to_value(&case["set"]);
                let type_name = value.reflect_type_name().to_string();
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Sequence(s) => match op {
                        "set" => s.reflect_set(
                            case["index"].as_u64().unwrap() as usize,
                            Box::new(new_value),
                        ),
                        _ => s.reflect_push(Box::new(new_value)),
                    },
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: if op == "set" {
                            "reflect_set"
                        } else {
                            "reflect_push"
                        },
                        type_name,
                    }),
                };
                match case.get("expect_error") {
                    Some(expected_error) => assert_eq!(
                        error_kind(&result.unwrap_err()),
                        expected_error.as_str().unwrap(),
                        "{}",
                        ctx()
                    ),
                    None => {
                        result.unwrap();
                        assert_eq!(value, support::to_value(&case["expected"]), "{}", ctx());
                    }
                }
            }
            other => panic!("unknown sequence op '{other}' in case '{name}'"),
        }
    }
}
