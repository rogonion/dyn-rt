//! Data-driven suite for the Value model (ADR 0004): one custom loop over a
//! plain Rust case table; every case carries a test_title. The table
//! doubles as usage documentation for building and inspecting Values.

use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

/// One assertion against the case's value.
enum Check {
    Type(&'static str),
    IsNull(bool),
    AsBool(bool),
    AsF64(f64),
    AsStr(&'static str),
    IsInteger(bool),
    AsArrayLen(Option<usize>),
    Get(&'static str, Value),
    GetMissing(&'static str),
    ExpectedObject(Value),
}

struct ValueCase {
    test_title: Option<String>,
    value: Value,
    checks: Vec<Check>,
}

macro_rules! value_case {
    ($value:expr, [$($check:expr),* $(,)?] $(, test_title = $title:expr)?) => {
        ValueCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            checks: vec![$($check),*],
        }
    };
}

#[test]
fn value_ops() {
    let test_data = vec![
        value_case!(
            Value::Null,
            [Check::Type("null"), Check::IsNull(true)],
            test_title = "null: type name and is_null"
        ),
        value_case!(
            Value::from(true),
            [Check::Type("boolean"), Check::AsBool(true)],
            test_title = "bool: type name and as_bool"
        ),
        value_case!(
            Value::from(1i64),
            [
                Check::Type("number"),
                Check::IsInteger(true),
                Check::AsF64(1.0)
            ],
            test_title = "int: type name, integer, as_f64"
        ),
        value_case!(
            Value::from(1.5f64),
            [
                Check::Type("number"),
                Check::IsInteger(false),
                Check::AsF64(1.5)
            ],
            test_title = "float: type name, not integer"
        ),
        value_case!(
            Value::from(18446744073709551615u64),
            [
                Check::Type("number"),
                Check::IsInteger(true),
                Check::AsF64(18446744073709551616.0)
            ],
            test_title = "u64 max stays lossless (never routed through f64 in the model)"
        ),
        value_case!(
            Value::from(-5i64),
            [
                Check::Type("number"),
                Check::IsInteger(true),
                Check::AsF64(-5.0)
            ],
            test_title = "negative int"
        ),
        value_case!(
            Value::from("x"),
            [
                Check::Type("string"),
                Check::AsStr("x"),
                Check::AsArrayLen(None)
            ],
            test_title = "string: as_str works, array accessor is gated off"
        ),
        value_case!(
            Value::from(vec![Value::from(1i64), Value::from(2i64)]),
            [Check::Type("array"), Check::AsArrayLen(Some(2))],
            test_title = "array: type name and length"
        ),
        value_case!(
            // duplicate key: last write wins, key order kept
            Value::object([
                ("a", Value::from(1i64)),
                ("b", Value::from("two")),
                ("a", Value::from(3i64)),
            ]),
            [
                Check::Type("object"),
                Check::Get("a", Value::from(3i64)),
                Check::Get("b", Value::from("two")),
                Check::GetMissing("zz"),
                Check::ExpectedObject(Value::object([
                    ("a", Value::from(3i64)),
                    ("b", Value::from("two")),
                ])),
            ],
            test_title = "object from entries: key order kept, duplicate keys replaced"
        ),
    ];

    let mut report = TestReport::new(
        "value_ops",
        "Data-driven suite for the Value model (ADR 0004): one custom loop over a",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        for check in &case.checks {
            let mismatch: Option<String> = match check {
                Check::Type(expected) => (case.value.type_name() != *expected).then(|| {
                    format!(
                        "type: expected {expected:?}, got {:?}",
                        case.value.type_name()
                    )
                }),
                Check::IsNull(expected) => (case.value.is_null() != *expected).then(|| {
                    format!(
                        "is_null: expected {expected:?}, got {:?}",
                        case.value.is_null()
                    )
                }),
                Check::AsBool(expected) => (case.value.as_bool() != Some(*expected)).then(|| {
                    format!(
                        "as_bool: expected {expected:?}, got {:?}",
                        case.value.as_bool()
                    )
                }),
                Check::AsF64(expected) => (case.value.as_f64() != Some(*expected)).then(|| {
                    format!(
                        "as_f64: expected {expected:?}, got {:?}",
                        case.value.as_f64()
                    )
                }),
                Check::AsStr(expected) => (case.value.as_str() != Some(*expected)).then(|| {
                    format!(
                        "as_str: expected {expected:?}, got {:?}",
                        case.value.as_str()
                    )
                }),
                Check::IsInteger(expected) => {
                    let got = case.value.as_number().map(|n| n.is_integer());
                    (got != Some(*expected))
                        .then(|| format!("is_integer: expected {expected:?}, got {got:?}"))
                }
                Check::AsArrayLen(expected) => {
                    let got = case.value.as_array().map(<[Value]>::len);
                    (got != *expected)
                        .then(|| format!("as_array_len: expected {expected:?}, got {got:?}"))
                }
                Check::Get(key, expected) => {
                    let got = case.value.get(key);
                    (got != Some(expected))
                        .then(|| format!("get {key:?}: expected {expected:?}, got {got:?}"))
                }
                Check::GetMissing(key) => (case.value.get(key).is_some()).then(|| {
                    format!(
                        "get_missing {key:?}: expected no entry, got {:?}",
                        case.value.get(key)
                    )
                }),
                Check::ExpectedObject(expected) => (&case.value != expected).then(|| {
                    format!(
                        "expected_object: expected {expected:?}, got {:?}",
                        case.value
                    )
                }),
            };
            if let Some(detail) = mismatch {
                report.fail_case(detail);
            }
        }
    }

    report.finish();
}
