//! Data-driven suites for the reflect module on domain Values (ADR 0004):
//! one custom loop per suite over a plain Rust table, every case carrying a
//! TestMetadata title. The tables double as usage examples for the reflect API.

use dyn_rt_reflect::{
    Reflect, ReflectError, ReflectKind, ReflectRefMut, as_enum, as_map, as_sequence, kind_of,
    reflect_with_inner,
};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

/// Build an object value from entries (key order kept, duplicate keys replace).
fn obj(entries: Vec<(&str, Value)>) -> Value {
    Value::object(entries)
}

/// Build an array value.
fn arr(items: Vec<Value>) -> Value {
    Value::from(items)
}

fn value_of(reflected: &dyn Reflect) -> &Value {
    reflected
        .reflect_as_any()
        .downcast_ref::<Value>()
        .expect("reflected value is a Value")
}

fn error_kind(err: &ReflectError) -> &'static str {
    match err {
        ReflectError::UnsupportedOperation { .. } => "UnsupportedOperation",
        ReflectError::KeyNotFound { .. } => "KeyNotFound",
        ReflectError::IndexOutOfBounds { .. } => "IndexOutOfBounds",
        ReflectError::ShapeMismatch { .. } => "ShapeMismatch",
        ReflectError::InnerBorrowFailed { .. } => "InnerBorrowFailed",
        ReflectError::InnerNotProvided { .. } => "InnerNotProvided",
        ReflectError::UnknownVariant { .. } => "UnknownVariant",
        _ => "Other",
    }
}

// ---------------------------------------------------------------------------
// kind
// ---------------------------------------------------------------------------

struct KindCase {
    meta: TestMetadata,
    value: Value,
    expect_kind: ReflectKind,
    expect_type: &'static str,
}

macro_rules! kind_case {
    ($value:expr, $expect_kind:expr, $expect_type:literal $(, test_title = $title:expr)?) => {
        KindCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            value: $value,
            expect_kind: $expect_kind,
            expect_type: $expect_type,
        }
    };
}

#[test]
fn reflect_kind() {
    let test_data: Vec<_> = vec![
        kind_case!(
            obj(vec![("a", 1i64.into())]),
            ReflectKind::Map,
            "object",
            test_title = "object is a map"
        ),
        kind_case!(
            arr(vec![1i64.into(), 2i64.into()]),
            ReflectKind::Sequence,
            "array",
            test_title = "array is a sequence"
        ),
        kind_case!(
            Value::from("x"),
            ReflectKind::Scalar,
            "string",
            test_title = "string is a scalar"
        ),
        kind_case!(
            Value::from(1i64),
            ReflectKind::Scalar,
            "number",
            test_title = "int is a scalar"
        ),
        kind_case!(
            Value::from(1.5f64),
            ReflectKind::Scalar,
            "number",
            test_title = "float is a scalar"
        ),
        kind_case!(
            Value::from(true),
            ReflectKind::Scalar,
            "boolean",
            test_title = "bool is a scalar"
        ),
        kind_case!(
            Value::Null,
            ReflectKind::Scalar,
            "null",
            test_title = "null is a scalar"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let reflected: &dyn Reflect = &case.value;
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        if kind_of(reflected) != case.expect_kind {
            failures.push(format!(
                "{title} || kind: expected {:?}, got {:?}",
                case.expect_kind,
                kind_of(reflected)
            ));
        }
        if reflected.reflect_type_name() != case.expect_type {
            failures.push(format!(
                "{title} || type name: expected {:?}, got {:?}",
                case.expect_type,
                reflected.reflect_type_name()
            ));
        }
    }
    report_failures(failures, test_data.len());
}

// ---------------------------------------------------------------------------
// map ops
// ---------------------------------------------------------------------------

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
    meta: TestMetadata,
    value: Value,
    op: MapOp,
}

macro_rules! map_case {
    ($value:expr, $op:expr $(, test_title = $title:expr)?) => {
        MapCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            value: $value,
            op: $op,
        }
    };
}

#[test]
fn reflect_map_ops() {
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
            arr(vec![1i64.into(), 2i64.into()]),
            MapOp::Get {
                key: "a",
                expected: None
            },
            test_title = "map get on a sequence fails"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut value = case.value.clone();
        let reflected: &dyn Reflect = &value;

        match &case.op {
            MapOp::Get { key, expected } => {
                let key = key.to_string();
                let result = as_map(reflected).and_then(|m| m.reflect_get(&key));
                let got = result.map(value_of);
                if got != expected.as_ref() {
                    failures.push(format!(
                        "{title} || get {key:?}: expected {expected:?}, got {got:?}"
                    ));
                }
            }
            MapOp::Contains { key, expected } => {
                let key = key.to_string();
                let map = as_map(reflected).expect("map expected");
                if map.reflect_contains_key(&key) != *expected {
                    failures.push(format!(
                        "{title} || contains {key:?}: expected {expected}, got {}",
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
                        other => failures.push(format!(
                            "{title} || set {key:?}: expected error {expected_error}, got {other:?}"
                        )),
                    },
                    None => {
                        match result {
                            Ok(()) => {
                                if Some(&value) != expected.as_ref() {
                                    failures.push(format!("{title} || set {key:?}: expected {expected:?}, got {value:?}"));
                                }
                            }
                            Err(err) => failures.push(format!(
                                "{title} || set {key:?}: unexpected error {}",
                                error_kind(&err)
                            )),
                        }
                    }
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
                        other => failures.push(format!("{title} || delete {key:?}: expected error {expected_error}, got {other:?}")),
                    },
                    None => match result {
                        Ok(()) => {
                            if Some(&value) != expected.as_ref() {
                                failures.push(format!("{title} || delete {key:?}: expected {expected:?}, got {value:?}"));
                            }
                        }
                        Err(err) => failures.push(format!("{title} || delete {key:?}: unexpected error {}", error_kind(&err))),
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
                    failures.push(format!(
                        "{title} || keys: expected {expected:?}, got {keys:?}"
                    ));
                }
            }
            MapOp::Len(expected) => {
                let map = as_map(reflected).expect("map expected");
                if map.reflect_length() != *expected {
                    failures.push(format!(
                        "{title} || len: expected {expected}, got {}",
                        map.reflect_length()
                    ));
                }
            }
        }
    }
    report_failures(failures, test_data.len());
}

// ---------------------------------------------------------------------------
// sequence ops
// ---------------------------------------------------------------------------

enum SeqOp {
    Get {
        index: usize,
        expected: Option<Value>,
    },
    Len(usize),
    Set {
        index: usize,
        set: Value,
        expected: Option<Value>,
        expect_error: Option<&'static str>,
    },
    Push {
        set: Value,
        expected: Option<Value>,
        expect_error: Option<&'static str>,
    },
    Delete {
        index: usize,
        expected: Option<Value>,
        expect_error: Option<&'static str>,
    },
}

struct SeqCase {
    meta: TestMetadata,
    value: Value,
    op: SeqOp,
}

macro_rules! seq_case {
    ($value:expr, $op:expr $(, test_title = $title:expr)?) => {
        SeqCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            value: $value,
            op: $op,
        }
    };
}

/// Apply a mutating sequence op and check the result: either an expected
/// error kind, or the expected value after the mutation.
fn run_mutating_seq_op(value: &mut Value, op: &SeqOp) -> Result<Result<(), &'static str>, ()> {
    let type_name = value.reflect_type_name().to_string();
    let result = match value.reflect_ref_mut() {
        ReflectRefMut::Sequence(s) => match op {
            SeqOp::Set { index, set, .. } => s.reflect_set_index(*index, Box::new(set.clone())),
            SeqOp::Push { set, .. } => s.reflect_push(Box::new(set.clone())),
            SeqOp::Delete { index, .. } => s.reflect_delete_index(*index),
            _ => unreachable!("not a mutating op"),
        },
        _ => Err(ReflectError::UnsupportedOperation {
            operation: "sequence op",
            type_name,
        }),
    };
    Ok(match result {
        Ok(()) => Ok(()),
        Err(err) => Err(error_kind(&err)),
    })
}

#[test]
fn reflect_sequence_ops() {
    let test_data: Vec<_> = vec![
        seq_case!(
            arr(vec![10i64.into(), 20i64.into(), 30i64.into()]),
            SeqOp::Get {
                index: 1,
                expected: Some(20i64.into())
            },
            test_title = "sequence get"
        ),
        seq_case!(
            arr(vec![10i64.into()]),
            SeqOp::Get {
                index: 5,
                expected: None
            },
            test_title = "sequence get out of bounds"
        ),
        seq_case!(
            arr(vec![10i64.into(), 20i64.into()]),
            SeqOp::Len(2),
            test_title = "sequence len"
        ),
        seq_case!(
            arr(vec![10i64.into(), 20i64.into()]),
            SeqOp::Set {
                index: 0,
                set: 99i64.into(),
                expected: Some(arr(vec![99i64.into(), 20i64.into()])),
                expect_error: None
            },
            test_title = "sequence set replaces"
        ),
        seq_case!(
            arr(vec![10i64.into()]),
            SeqOp::Set {
                index: 3,
                set: 1i64.into(),
                expected: None,
                expect_error: Some("IndexOutOfBounds")
            },
            test_title = "sequence set out of bounds fails"
        ),
        seq_case!(
            arr(vec![10i64.into()]),
            SeqOp::Push {
                set: 20i64.into(),
                expected: Some(arr(vec![10i64.into(), 20i64.into()])),
                expect_error: None
            },
            test_title = "sequence push appends"
        ),
        seq_case!(
            arr(vec![10i64.into(), 20i64.into(), 30i64.into()]),
            SeqOp::Delete {
                index: 1,
                expected: Some(arr(vec![10i64.into(), 30i64.into()])),
                expect_error: None
            },
            test_title = "sequence delete shifts elements"
        ),
        seq_case!(
            obj(vec![("a", 1i64.into())]),
            SeqOp::Push {
                set: 20i64.into(),
                expected: None,
                expect_error: Some("UnsupportedOperation")
            },
            test_title = "sequence push on a map fails"
        ),
        seq_case!(
            Value::from(5i64),
            SeqOp::Get {
                index: 0,
                expected: None
            },
            test_title = "sequence get on a scalar"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut value = case.value.clone();
        let reflected: &dyn Reflect = &value;

        match &case.op {
            SeqOp::Get { index, expected } => {
                let result = as_sequence(reflected).and_then(|s| s.reflect_get(*index));
                let got = result.map(value_of);
                if got != expected.as_ref() {
                    failures.push(format!(
                        "{title} || get [{index}]: expected {expected:?}, got {got:?}"
                    ));
                }
            }
            SeqOp::Len(expected) => {
                let seq = as_sequence(reflected).expect("sequence expected");
                if seq.reflect_length() != *expected {
                    failures.push(format!(
                        "{title} || len: expected {expected}, got {}",
                        seq.reflect_length()
                    ));
                }
            }
            SeqOp::Set {
                index,
                expected,
                expect_error,
                ..
            } => match run_mutating_seq_op(&mut value, &case.op) {
                Ok(Ok(())) => {
                    if Some(&value) != expected.as_ref() {
                        failures.push(format!(
                            "{title} || set [{index}]: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Ok(Err(got)) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => failures.push(format!(
                        "{title} || set [{index}]: expected error {expect_error:?}, got {got:?}"
                    )),
                },
                Err(()) => failures.push(format!("{title} || set [{index}]: op setup failed")),
            },
            SeqOp::Push {
                expected,
                expect_error,
                ..
            } => match run_mutating_seq_op(&mut value, &case.op) {
                Ok(Ok(())) => {
                    if Some(&value) != expected.as_ref() {
                        failures.push(format!(
                            "{title} || push: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Ok(Err(got)) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => failures.push(format!(
                        "{title} || push: expected error {expect_error:?}, got {got:?}"
                    )),
                },
                Err(()) => failures.push(format!("{title} || push: op setup failed")),
            },
            SeqOp::Delete {
                index,
                expected,
                expect_error,
            } => match run_mutating_seq_op(&mut value, &case.op) {
                Ok(Ok(())) => {
                    if Some(&value) != expected.as_ref() {
                        failures.push(format!(
                            "{title} || delete [{index}]: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Ok(Err(got)) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => failures.push(format!(
                        "{title} || delete [{index}]: expected error {expect_error:?}, got {got:?}"
                    )),
                },
                Err(()) => failures.push(format!("{title} || delete [{index}]: op setup failed")),
            },
        }
    }
    report_failures(failures, test_data.len());
}

// ---------------------------------------------------------------------------
// clone / apply
// ---------------------------------------------------------------------------

enum CloneApplyOp {
    Clone {
        expected: Value,
    },
    Apply {
        source: Value,
        expected: Option<Value>,
        expect_error: Option<&'static str>,
    },
    ApplyString {
        source: &'static str,
        expect_error: &'static str,
    },
}

struct CloneApplyCase {
    meta: TestMetadata,
    value: Value,
    op: CloneApplyOp,
}

macro_rules! clone_apply_case {
    ($value:expr, $op:expr $(, test_title = $title:expr)?) => {
        CloneApplyCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            value: $value,
            op: $op,
        }
    };
}

#[test]
fn reflect_clone_apply() {
    let test_data: Vec<_> = vec![
        clone_apply_case!(
            obj(vec![("a", arr(vec![1i64.into(), 2i64.into()]))]),
            CloneApplyOp::Clone {
                expected: obj(vec![("a", arr(vec![1i64.into(), 2i64.into()]))])
            },
            test_title = "clone of object is deep and equal"
        ),
        clone_apply_case!(
            Value::from(5i64),
            CloneApplyOp::Clone {
                expected: 5i64.into()
            },
            test_title = "clone of scalar"
        ),
        clone_apply_case!(
            arr(vec![1i64.into(), "x".into()]),
            CloneApplyOp::Clone {
                expected: arr(vec![1i64.into(), "x".into()])
            },
            test_title = "clone of array"
        ),
        clone_apply_case!(
            obj(vec![("a", 1i64.into())]),
            CloneApplyOp::Apply {
                source: obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
                expected: Some(obj(vec![("a", 1i64.into()), ("b", 2i64.into())])),
                expect_error: None
            },
            test_title = "apply copies structurally"
        ),
        clone_apply_case!(
            obj(vec![("a", 1i64.into())]),
            CloneApplyOp::ApplyString {
                source: "hello",
                expect_error: "ShapeMismatch"
            },
            test_title = "apply with wrong concrete type fails"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut value = case.value.clone();

        match &case.op {
            CloneApplyOp::Clone { expected } => {
                let reflected: &dyn Reflect = &value;
                let cloned = reflected.reflect_clone();
                let got = value_of(cloned.as_ref());
                if got != expected {
                    failures.push(format!(
                        "{title} || clone: expected {expected:?}, got {got:?}"
                    ));
                }
            }
            CloneApplyOp::Apply {
                source,
                expected,
                expect_error,
            } => {
                let source_ref: &dyn Reflect = source;
                let result = value.reflect_apply(source_ref);
                match expect_error {
                    Some(expected_error) => match &result {
                        Err(err) if error_kind(err) == *expected_error => {}
                        other => failures.push(format!(
                            "{title} || apply: expected error {expected_error}, got {other:?}"
                        )),
                    },
                    None => match result {
                        Ok(()) => {
                            if Some(&value) != expected.as_ref() {
                                failures.push(format!(
                                    "{title} || apply: expected {expected:?}, got {value:?}"
                                ));
                            }
                        }
                        Err(err) => failures.push(format!(
                            "{title} || apply: unexpected error {}",
                            error_kind(&err)
                        )),
                    },
                }
            }
            CloneApplyOp::ApplyString {
                source,
                expect_error,
            } => {
                let source_string = source.to_string();
                let source_ref: &dyn Reflect = &source_string;
                let result = value.reflect_apply(source_ref);
                match &result {
                    Err(err) if error_kind(err) == *expect_error => {}
                    other => failures.push(format!(
                        "{title} || apply string: expected error {expect_error}, got {other:?}"
                    )),
                }
            }
        }
    }
    report_failures(failures, test_data.len());
}

// ---------------------------------------------------------------------------
// indirection / enum
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Subject {
    Boxed(Value),
    Some(Value),
    None,
}

impl Subject {
    fn into_reflected(self) -> Box<dyn Reflect> {
        match self {
            Subject::Boxed(v) => Box::new(Box::new(v)),
            Subject::Some(v) => Box::new(Some(v)),
            Subject::None => Box::new(None::<Value>),
        }
    }
}

enum IndOp {
    Kind(&'static str),
    Inner(Value),
    SetInner {
        set: Value,
        expected: Value,
    },
    DeleteInner {
        expect_error: &'static str,
    },
    Variant(&'static str),
    Payload(Value),
    SetVariant {
        variant: &'static str,
        expected_none: bool,
        expect_error: Option<&'static str>,
    },
}

struct IndCase {
    meta: TestMetadata,
    subject: Subject,
    op: IndOp,
}

macro_rules! ind_case {
    ($subject:expr, $op:expr $(, test_title = $title:expr)?) => {
        IndCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            subject: $subject,
            op: $op,
        }
    };
}

#[test]
fn reflect_indirection_enum() {
    let test_data: Vec<_> = vec![
        ind_case!(
            Subject::Boxed(obj(vec![("a", 1i64.into())])),
            IndOp::Inner(obj(vec![("a", 1i64.into())])),
            test_title = "box inner read"
        ),
        ind_case!(
            Subject::Boxed(obj(vec![("a", 1i64.into())])),
            IndOp::SetInner {
                set: obj(vec![("b", 2i64.into())]),
                expected: obj(vec![("b", 2i64.into())])
            },
            test_title = "box set_inner replaces"
        ),
        ind_case!(
            Subject::Boxed(1i64.into()),
            IndOp::DeleteInner {
                expect_error: "UnsupportedOperation"
            },
            test_title = "box delete_inner unsupported"
        ),
        ind_case!(
            Subject::Boxed(1i64.into()),
            IndOp::Kind("indirection"),
            test_title = "box kind is indirection"
        ),
        ind_case!(
            Subject::Some(5i64.into()),
            IndOp::Variant("Some"),
            test_title = "option some reports Some variant"
        ),
        ind_case!(
            Subject::None,
            IndOp::Variant("None"),
            test_title = "option none reports None variant"
        ),
        ind_case!(
            Subject::Some(5i64.into()),
            IndOp::Payload(5i64.into()),
            test_title = "option some payload readable"
        ),
        ind_case!(
            Subject::Some(5i64.into()),
            IndOp::SetVariant {
                variant: "None",
                expected_none: true,
                expect_error: None
            },
            test_title = "option set_variant to None"
        ),
        ind_case!(
            Subject::Some(5i64.into()),
            IndOp::SetVariant {
                variant: "Maybe",
                expected_none: false,
                expect_error: Some("UnknownVariant")
            },
            test_title = "option set_variant unknown fails"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut subject = case.subject.clone().into_reflected();

        match &case.op {
            IndOp::Kind(expected) => {
                if kind_of(subject.as_ref()).to_string() != *expected {
                    failures.push(format!(
                        "{title} || kind: expected {expected:?}, got {:?}",
                        kind_of(subject.as_ref())
                    ));
                }
            }
            IndOp::Inner(expected) => {
                let result = reflect_with_inner(subject.as_ref(), |v| value_of(v).clone());
                match result {
                    Ok(inner) if &inner == expected => {}
                    Ok(inner) => failures.push(format!(
                        "{title} || inner: expected {expected:?}, got {inner:?}"
                    )),
                    Err(err) => failures.push(format!(
                        "{title} || inner: unexpected error {}",
                        error_kind(&err)
                    )),
                }
            }
            IndOp::SetInner { set, expected } => match subject.reflect_ref_mut() {
                ReflectRefMut::Indirection(ind) => match ind
                    .reflect_set_inner(Box::new(set.clone()))
                {
                    Ok(()) => {
                        let inner = reflect_with_inner(subject.as_ref(), |v| value_of(v).clone());
                        match inner {
                            Ok(inner) if &inner == expected => {}
                            Ok(inner) => failures.push(format!(
                                "{title} || set_inner: expected {expected:?}, got {inner:?}"
                            )),
                            Err(err) => failures.push(format!(
                                "{title} || set_inner read-back: {}",
                                error_kind(&err)
                            )),
                        }
                    }
                    Err(err) => failures.push(format!(
                        "{title} || set_inner: unexpected error {}",
                        error_kind(&err)
                    )),
                },
                _ => failures.push(format!("{title} || set_inner: expected indirection")),
            },
            IndOp::DeleteInner { expect_error } => {
                let result = match subject.reflect_ref_mut() {
                    ReflectRefMut::Indirection(ind) => ind.reflect_delete_inner(),
                    _ => {
                        failures.push(format!("{title} || delete_inner: expected indirection"));
                        continue;
                    }
                };
                match &result {
                    Err(err) if error_kind(err) == *expect_error => {}
                    other => failures.push(format!(
                        "{title} || delete_inner: expected error {expect_error}, got {other:?}"
                    )),
                }
            }
            IndOp::Variant(expected) => {
                let enumeration = as_enum(subject.as_ref()).expect("enum expected");
                if enumeration.reflect_active_variant() != *expected {
                    failures.push(format!(
                        "{title} || variant: expected {expected:?}, got {:?}",
                        enumeration.reflect_active_variant()
                    ));
                }
            }
            IndOp::Payload(expected) => {
                let enumeration = as_enum(subject.as_ref()).expect("enum expected");
                let payload = enumeration
                    .reflect_active_variant_payload()
                    .map(value_of)
                    .expect("payload expected");
                if payload != expected {
                    failures.push(format!(
                        "{title} || payload: expected {expected:?}, got {payload:?}"
                    ));
                }
            }
            IndOp::SetVariant {
                variant,
                expected_none,
                expect_error,
            } => {
                let result = match subject.reflect_ref_mut() {
                    ReflectRefMut::Enum(e) => e.reflect_set_variant(variant, None),
                    _ => {
                        failures.push(format!("{title} || set_variant: expected enum"));
                        continue;
                    }
                };
                match expect_error {
                    Some(expected_error) => match &result {
                        Err(err) if error_kind(err) == *expected_error => {}
                        other => failures.push(format!("{title} || set_variant {variant:?}: expected error {expected_error}, got {other:?}")),
                    },
                    None => match result {
                        Ok(()) => {
                            let enumeration = as_enum(subject.as_ref()).expect("enum");
                            if enumeration.reflect_active_variant() != *variant {
                                failures.push(format!("{title} || set_variant {variant:?}: variant is {:?}", enumeration.reflect_active_variant()));
                            }
                            if *expected_none && enumeration.reflect_active_variant_payload().is_some() {
                                failures.push(format!("{title} || set_variant {variant:?}: expected no payload"));
                            }
                        }
                        Err(err) => failures.push(format!("{title} || set_variant {variant:?}: unexpected error {}", error_kind(&err))),
                    },
                }
            }
        }
    }
    report_failures(failures, test_data.len());
}
