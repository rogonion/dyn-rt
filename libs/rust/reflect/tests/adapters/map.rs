//! Adapter suite for `src/adapters/map.rs`: ReflectMap on Value objects.

use crate::common::{error_kind, obj, value_of};
use dyn_rt_reflect::{Reflect, ReflectError, ReflectRefMut, as_map};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

enum MapOp {
    Get {
        key: &'static str,
        expected: Option<Value>,
    },
    Contains {
        key: &'static str,
        expected: bool,
    },
    Set {
        key: &'static str,
        set: Value,
        expected: Option<Value>,
        expect_error: Option<&'static str>,
    },
    Delete {
        key: &'static str,
        expected: Option<Value>,
        expect_error: Option<&'static str>,
    },
    Keys(Vec<&'static str>),
    Len(usize),
}

struct MapCase {
    test_title: Option<String>,
    value: Value,
    op: MapOp,
}

macro_rules! map_case {
    ($value:expr, $op:expr $(, test_title = $title:expr)?) => {
        MapCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            op: $op,
        }
    };
}

#[test]
fn value_map_ops() {
    let test_data: Vec<_> = vec![
        map_case!(
            obj(vec![("a", 1i64.into()), ("b", "x".into())]),
            MapOp::Get {
                key: "a",
                expected: Some(1i64.into())
            },
            test_title = "map get existing key"
        ),
        map_case!(
            obj(vec![("a", 1i64.into())]),
            MapOp::Get {
                key: "zz",
                expected: None
            },
            test_title = "map get missing key"
        ),
        map_case!(
            obj(vec![("a", 1i64.into())]),
            MapOp::Contains {
                key: "a",
                expected: true
            },
            test_title = "map contains existing key"
        ),
        map_case!(
            obj(vec![("a", 1i64.into())]),
            MapOp::Contains {
                key: "zz",
                expected: false
            },
            test_title = "map contains missing key"
        ),
        map_case!(
            obj(vec![("a", 1i64.into())]),
            MapOp::Set {
                key: "b",
                set: 2i64.into(),
                expected: Some(obj(vec![("a", 1i64.into()), ("b", 2i64.into())])),
                expect_error: None
            },
            test_title = "map set new key appends"
        ),
        map_case!(
            obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
            MapOp::Set {
                key: "a",
                set: 9i64.into(),
                expected: Some(obj(vec![("a", 9i64.into()), ("b", 2i64.into())])),
                expect_error: None
            },
            test_title = "map set existing key replaces in place"
        ),
        map_case!(
            obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
            MapOp::Delete {
                key: "a",
                expected: Some(obj(vec![("b", 2i64.into())])),
                expect_error: None
            },
            test_title = "map delete existing key"
        ),
        map_case!(
            obj(vec![("a", 1i64.into())]),
            MapOp::Delete {
                key: "zz",
                expected: None,
                expect_error: Some("KeyNotFound")
            },
            test_title = "map delete missing key errors KeyNotFound"
        ),
        map_case!(
            obj(vec![("b", 1i64.into()), ("a", 2i64.into())]),
            MapOp::Keys(vec!["b", "a"]),
            test_title = "map keys keep insertion order"
        ),
        map_case!(
            obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
            MapOp::Len(2),
            test_title = "map len"
        ),
        map_case!(
            Value::from(1i64),
            MapOp::Set {
                key: "a",
                set: 2i64.into(),
                expected: None,
                expect_error: Some("UnsupportedOperation")
            },
            test_title = "map set on a scalar fails"
        ),
        map_case!(
            Value::from(vec![1i64.into(), 2i64.into()]),
            MapOp::Get {
                key: "a",
                expected: None
            },
            test_title = "map get on a sequence fails"
        ),
    ];

    let mut report = TestReport::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.test_title);
        report.begin_case(title.clone());
        let mut value = case.value.clone();
        let reflected: &dyn Reflect = &value;

        match &case.op {
            MapOp::Get { key, expected } => {
                let key = key.to_string();
                let result = as_map(reflected).and_then(|m| m.reflect_get(&key));
                let got = result.map(value_of);
                if got != expected.as_ref() {
                    report.fail_case(format!("get {key:?}: expected {expected:?}, got {got:?}"));
                }
            }
            MapOp::Contains { key, expected } => {
                let key = key.to_string();
                let map = as_map(reflected).expect("map expected");
                if map.reflect_contains_key(&key) != *expected {
                    report.fail_case(format!(
                        "contains {key:?}: expected {expected}, got {}",
                        map.reflect_contains_key(&key)
                    ));
                }
            }
            MapOp::Set {
                key,
                set,
                expected,
                expect_error,
            } => {
                let key = key.to_string();
                let type_name = value.reflect_type_name().to_string();
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Map(m) => m.reflect_set_key(&key, Box::new(set.clone())),
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: "reflect_set_key",
                        type_name,
                    }),
                };
                match expect_error {
                    Some(expected_error) => match &result {
                        Err(err) if error_kind(err) == *expected_error => {}
                        other => report.fail_case(format!(
                            "set {key:?}: expected error {expected_error}, got {other:?}"
                        )),
                    },
                    None => match result {
                        Ok(()) => {
                            if Some(&value) != expected.as_ref() {
                                report.fail_case(format!(
                                    "set {key:?}: expected {expected:?}, got {value:?}"
                                ));
                            }
                        }
                        Err(err) => report.fail_case(format!(
                            "set {key:?}: unexpected error {}",
                            error_kind(&err)
                        )),
                    },
                }
            }
            MapOp::Delete {
                key,
                expected,
                expect_error,
            } => {
                let key = key.to_string();
                let type_name = value.reflect_type_name().to_string();
                let result = match value.reflect_ref_mut() {
                    ReflectRefMut::Map(m) => m.reflect_delete_key(&key),
                    _ => Err(ReflectError::UnsupportedOperation {
                        operation: "reflect_delete_key",
                        type_name,
                    }),
                };
                match expect_error {
                    Some(expected_error) => match &result {
                        Err(err) if error_kind(err) == *expected_error => {}
                        other => report.fail_case(format!(
                            "delete {key:?}: expected error {expected_error}, got {other:?}"
                        )),
                    },
                    None => match result {
                        Ok(()) => {
                            if Some(&value) != expected.as_ref() {
                                report.fail_case(format!(
                                    "delete {key:?}: expected {expected:?}, got {value:?}"
                                ));
                            }
                        }
                        Err(err) => report.fail_case(format!(
                            "delete {key:?}: unexpected error {}",
                            error_kind(&err)
                        )),
                    },
                }
            }
            MapOp::Keys(expected) => {
                let map = as_map(reflected).expect("map expected");
                let keys: Vec<String> = map
                    .reflect_iter_keys()
                    .map(|k| {
                        k.reflect_as_any()
                            .downcast_ref::<String>()
                            .expect("string key")
                            .clone()
                    })
                    .collect();
                let expected: Vec<String> = expected.iter().map(|k| k.to_string()).collect();
                if keys != expected {
                    report.fail_case(format!("keys: expected {expected:?}, got {keys:?}"));
                }
            }
            MapOp::Len(expected) => {
                let map = as_map(reflected).expect("map expected");
                if map.reflect_length() != *expected {
                    report.fail_case(format!(
                        "len: expected {expected}, got {}",
                        map.reflect_length()
                    ));
                }
            }
        }
    }
    report.finish();
}
