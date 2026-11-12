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
        ReflectError::KeyNotFound { .. } => "KeyNotFound",
        ReflectError::IndexOutOfBounds { .. } => "IndexOutOfBounds",
        ReflectError::ShapeMismatch { .. } => "ShapeMismatch",
        ReflectError::UnknownVariant { .. } => "UnknownVariant",
        _ => "Other",
    }
}

fn string_key(key: &str) -> String {
    key.to_string()
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
                let key = string_key(case["key"].as_str().unwrap());
                let map = reflect::as_map(&value as &dyn Reflect);
                let result = map.and_then(|m| m.reflect_get(&key));
                let expected = support::to_value(&case["expected"]);
                match (&case["expected"], result) {
                    (serde_json::Value::Null, None) => {}
                    _ => assert_eq!(result.map(support::value_of), Some(&expected), "{}", ctx()),
                }
            }
            "contains" => {
                let key = string_key(case["key"].as_str().unwrap());
                let map = reflect::as_map(&value as &dyn Reflect).expect("map expected");
                assert_eq!(
                    map.reflect_contains_key(&key),
                    case["expected"].as_bool().unwrap(),
                    "{}",
                    ctx()
                );
            }
            "set" => {
                let key = string_key(case["key"].as_str().unwrap());
                let new_value: Value = support::to_value(&case["set"]);
                let type_name = value.reflect_type_name().to_string();
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Map(m) => m.reflect_set_key(&key, Box::new(new_value)),
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: "reflect_set_key",
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
                let key = string_key(case["key"].as_str().unwrap());
                let type_name = value.reflect_type_name().to_string();
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Map(m) => m.reflect_delete_key(&key),
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: "reflect_delete_key",
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
            "keys" => {
                let map = reflect::as_map(&value as &dyn Reflect).expect("map expected");
                let keys: Vec<String> = map
                    .reflect_iter_keys()
                    .map(|k| {
                        k.reflect_as_any()
                            .downcast_ref::<String>()
                            .expect("string key")
                            .clone()
                    })
                    .collect();
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
                    map.reflect_length(),
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
                    seq.reflect_length(),
                    case["expected"].as_u64().unwrap() as usize,
                    "{}",
                    ctx()
                );
            }
            "set" | "push" | "delete" => {
                let new_value: Value = support::to_value(&case["set"]);
                let type_name = value.reflect_type_name().to_string();
                let op_name = if op == "set" {
                    "reflect_set_index"
                } else if op == "push" {
                    "reflect_push"
                } else {
                    "reflect_delete_index"
                };
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Sequence(s) => match op {
                        "set" => s.reflect_set_index(
                            case["index"].as_u64().unwrap() as usize,
                            Box::new(new_value),
                        ),
                        "push" => s.reflect_push(Box::new(new_value)),
                        _ => s.reflect_delete_index(case["index"].as_u64().unwrap() as usize),
                    },
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: op_name,
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

#[test]
fn reflect_clone_apply() {
    for case in support::load_cases("reflect/clone-apply.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let mut value = support::to_value(&case["value"]);
        let op = case["op"].as_str().unwrap();
        let ctx = || ctx(&case, name);

        match op {
            "clone" => {
                let reflected: &dyn Reflect = &value;
                let cloned = reflected.reflect_clone();
                assert_eq!(
                    support::value_of(cloned.as_ref()),
                    &support::to_value(&case["expected"]),
                    "{}",
                    ctx()
                );
            }
            "apply" => {
                let source_string;
                let source_value;
                let source: &dyn Reflect = if let Some(s) = case.get("source_string") {
                    source_string = s.as_str().unwrap().to_string();
                    &source_string
                } else {
                    source_value = support::to_value(&case["source"]);
                    &source_value
                };
                let result = value.reflect_apply(source);
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
            other => panic!("unknown clone/apply op '{other}' in case '{name}'"),
        }
    }
}

#[test]
fn reflect_indirection_enum() {
    for case in support::load_cases("reflect/indirection-enum.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let op = case["op"].as_str().unwrap();
        let ctx = || ctx(&case, name);

        // build the subject as a boxed reflected value
        let build = &case["build"];
        let subject: Box<dyn Reflect> = if let Some(inner) = build.get("box") {
            Box::new(Box::new(support::to_value(inner)))
        } else if let Some(inner) = build.get("some") {
            Box::new(Some(support::to_value(inner)))
        } else if build.get("none").is_some() {
            Box::new(None::<Value>)
        } else {
            panic!("bad build instruction in case '{name}'")
        };

        match op {
            "kind" => {
                assert_eq!(
                    reflect::kind_of(subject.as_ref()).to_string(),
                    case["expected"].as_str().unwrap(),
                    "{}",
                    ctx()
                );
            }
            "inner" => {
                let inner =
                    reflect::reflect_with_inner(subject.as_ref(), |v| support::value_of(v).clone())
                        .unwrap();
                assert_eq!(inner, support::to_value(&case["expected"]), "{}", ctx());
            }
            "set_inner" => {
                let mut subject = subject;
                let new_value: Value = support::to_value(&case["set"]);
                match subject.reflect_ref_mut() {
                    ReflectRefMut::Indirection(ind) => {
                        ind.reflect_set_inner(Box::new(new_value)).unwrap();
                    }
                    _ => panic!("expected indirection in case '{name}'"),
                }
                let inner =
                    reflect::reflect_with_inner(subject.as_ref(), |v| support::value_of(v).clone())
                        .unwrap();
                assert_eq!(inner, support::to_value(&case["expected"]), "{}", ctx());
            }
            "delete_inner" => {
                let mut subject = subject;
                let result = match subject.reflect_ref_mut() {
                    ReflectRefMut::Indirection(ind) => ind.reflect_delete_inner(),
                    _ => panic!("expected indirection in case '{name}'"),
                };
                assert_eq!(
                    error_kind(&result.unwrap_err()),
                    case["expect_error"].as_str().unwrap(),
                    "{}",
                    ctx()
                );
            }
            "variant" => {
                let enumeration = reflect::as_enum(subject.as_ref()).expect("enum expected");
                assert_eq!(
                    enumeration.reflect_active_variant(),
                    case["expected"].as_str().unwrap(),
                    "{}",
                    ctx()
                );
            }
            "payload" => {
                let enumeration = reflect::as_enum(subject.as_ref()).expect("enum expected");
                let payload = enumeration
                    .reflect_active_variant_payload()
                    .map(support::value_of)
                    .expect("payload expected");
                assert_eq!(payload, &support::to_value(&case["expected"]), "{}", ctx());
            }
            "set_variant" => {
                let mut subject = subject;
                let variant = case["variant"].as_str().unwrap();
                let result = match subject.reflect_ref_mut() {
                    ReflectRefMut::Enum(e) => e.reflect_set_variant(variant, None),
                    _ => panic!("expected enum in case '{name}'"),
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
                        let enumeration = reflect::as_enum(subject.as_ref()).expect("enum");
                        assert_eq!(enumeration.reflect_active_variant(), variant, "{}", ctx());
                        if case["expected_none"].as_bool() == Some(true) {
                            assert!(enumeration.reflect_active_variant_payload().is_none());
                        }
                    }
                }
            }
            other => panic!("unknown indirection/enum op '{other}' in case '{name}'"),
        }
    }
}
