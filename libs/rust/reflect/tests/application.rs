//! Data-driven suites for the reflect APPLICATION layer (ADR 0004): kind_of,
//! children, the as_* downcasts, reflect_with_inner. Subjects are domain
//! Values (adapter impls) since the domain itself defines no concrete types.

mod common;

use common::{arr, error_kind, obj, value_of};
use dyn_rt_reflect::{
    Reflect, ReflectKind, ReflectRef, as_enum, as_indirection, as_map, as_scalar, as_sequence,
    as_struct, as_unknown, children, kind_of, reflect_with_inner,
};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// kind_of
// ---------------------------------------------------------------------------

struct KindOfCase {
    test_title: Option<String>,
    value: Value,
    expect_kind: ReflectKind,
    expect_type: &'static str,
}

macro_rules! kind_of_case {
    ($value:expr, $expect_kind:expr, $expect_type:literal $(, test_title = $title:expr)?) => {
        KindOfCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            expect_kind: $expect_kind,
            expect_type: $expect_type,
        }
    };
}

#[test]
fn kind_of_cases() {
    let test_data: Vec<_> = vec![
        kind_of_case!(
            obj(vec![("a", 1i64.into())]),
            ReflectKind::Map,
            "object",
            test_title = "object is a map"
        ),
        kind_of_case!(
            arr(vec![1i64.into(), 2i64.into()]),
            ReflectKind::Sequence,
            "array",
            test_title = "array is a sequence"
        ),
        kind_of_case!(
            Value::from("x"),
            ReflectKind::Scalar,
            "string",
            test_title = "string is a scalar"
        ),
        kind_of_case!(
            Value::from(1i64),
            ReflectKind::Scalar,
            "number",
            test_title = "int is a scalar"
        ),
        kind_of_case!(
            Value::from(1.5f64),
            ReflectKind::Scalar,
            "number",
            test_title = "float is a scalar"
        ),
        kind_of_case!(
            Value::from(true),
            ReflectKind::Scalar,
            "boolean",
            test_title = "bool is a scalar"
        ),
        kind_of_case!(
            Value::Null,
            ReflectKind::Scalar,
            "null",
            test_title = "null is a scalar"
        ),
    ];

    let mut report = TestReport::new(
        "kind_of_cases",
        "Data-driven suites for the reflect APPLICATION layer (ADR 0004): kind_of,",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let reflected: &dyn Reflect = &case.value;
        if kind_of(reflected) != case.expect_kind {
            report.fail_case(format!(
                "kind: expected {:?}, got {:?}",
                case.expect_kind,
                kind_of(reflected)
            ));
        }
        if reflected.reflect_type_name() != case.expect_type {
            report.fail_case(format!(
                "type name: expected {:?}, got {:?}",
                case.expect_type,
                reflected.reflect_type_name()
            ));
        }
    }
    report.finish();
}

// ---------------------------------------------------------------------------
// children
// ---------------------------------------------------------------------------

struct ChildrenCase {
    test_title: Option<String>,
    value: Value,
    expected: Vec<Value>,
}

macro_rules! children_case {
    ($value:expr, $expected:expr $(, test_title = $title:expr)?) => {
        ChildrenCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            expected: $expected,
        }
    };
}

#[test]
fn children_of_containers() {
    let test_data: Vec<_> = vec![
        children_case!(
            obj(vec![("a", 1i64.into()), ("b", "x".into())]),
            vec![1i64.into(), "x".into()],
            test_title = "object children are values in key order"
        ),
        children_case!(
            arr(vec![1i64.into(), 2i64.into()]),
            vec![1i64.into(), 2i64.into()],
            test_title = "array children are elements in order"
        ),
        children_case!(
            Value::from(5i64),
            vec![],
            test_title = "scalar has no children"
        ),
    ];

    let mut report = TestReport::new(
        "children_of_containers",
        "Data-driven suites for the reflect APPLICATION layer (ADR 0004): kind_of,",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let reflected: &dyn Reflect = &case.value;
        let got: Vec<Value> = children(reflected)
            .iter()
            .map(|v| value_of(*v).clone())
            .collect();
        if got != case.expected {
            report.fail_case(format!("expected {:?}, got {:?}", case.expected, got,));
        }
    }
    report.finish();
}

// ---------------------------------------------------------------------------
// as_* downcasts
// ---------------------------------------------------------------------------

struct DowncastCase {
    test_title: Option<String>,
    value: Value,
    /// which as_* returns Some: "scalar" | "sequence" | "map" | "struct" |
    /// "enum" | "indirection" | "unknown"
    expected: &'static str,
}

macro_rules! downcast_case {
    ($value:expr, $expected:literal $(, test_title = $title:expr)?) => {
        DowncastCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            expected: $expected,
        }
    };
}

#[test]
fn as_downcasts() {
    let test_data: Vec<_> = vec![
        downcast_case!(
            obj(vec![("a", 1i64.into())]),
            "map",
            test_title = "object downcasts to map"
        ),
        downcast_case!(
            arr(vec![1i64.into()]),
            "sequence",
            test_title = "array downcasts to sequence"
        ),
        downcast_case!(
            Value::from(1i64),
            "scalar",
            test_title = "number downcasts to scalar"
        ),
        downcast_case!(
            Value::from("x"),
            "scalar",
            test_title = "string downcasts to scalar"
        ),
    ];

    let mut report = TestReport::new(
        "as_downcasts",
        "Data-driven suites for the reflect APPLICATION layer (ADR 0004): kind_of,",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let reflected: &dyn Reflect = &case.value;
        let got = if as_scalar(reflected).is_some() {
            "scalar"
        } else if as_sequence(reflected).is_some() {
            "sequence"
        } else if as_map(reflected).is_some() {
            "map"
        } else if as_struct(reflected).is_some() {
            "struct"
        } else if as_enum(reflected).is_some() {
            "enum"
        } else if as_indirection(reflected).is_some() {
            "indirection"
        } else if as_unknown(reflected).is_some() {
            "unknown"
        } else {
            "none"
        };
        if got != case.expected {
            report.fail_case(format!(
                "expected downcast {got:?} to be {:?}",
                case.expected
            ));
        }
    }
    report.finish();
}

// ---------------------------------------------------------------------------
// reflect_with_inner
// ---------------------------------------------------------------------------

enum Subject {
    Boxed(Value),
    Plain(Value),
}

struct WithInnerCase {
    test_title: Option<String>,
    subject: Subject,
    expected: Result<Value, &'static str>,
}

macro_rules! with_inner_case {
    ($subject:expr, $expected:expr $(, test_title = $title:expr)?) => {
        WithInnerCase {
test_title: None $(.or(Some($title.to_string())))?,
            subject: $subject,
            expected: $expected,
        }
    };
}

#[test]
fn reflect_with_inner_cases() {
    let test_data: Vec<_> = vec![
        with_inner_case!(
            Subject::Boxed(obj(vec![("a", 1i64.into())])),
            Ok(obj(vec![("a", 1i64.into())])),
            test_title = "boxed value reads inner"
        ),
        with_inner_case!(
            Subject::Plain(Value::from(5i64)),
            Err("UnsupportedOperation"),
            test_title = "non-indirection errors UnsupportedOperation"
        ),
    ];

    let mut report = TestReport::new(
        "reflect_with_inner_cases",
        "Data-driven suites for the reflect APPLICATION layer (ADR 0004): kind_of,",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let subject: Box<dyn Reflect> = match &case.subject {
            Subject::Boxed(v) => Box::new(Box::new(v.clone())),
            Subject::Plain(v) => Box::new(v.clone()),
        };
        let result = reflect_with_inner(subject.as_ref(), |v| value_of(v).clone());
        match (&case.expected, result) {
            (Ok(expected), Ok(got)) => {
                if &got != expected {
                    report.fail_case(format!("expected {expected:?}, got {got:?}"));
                }
            }
            (Err(expected_error), Err(err)) => {
                if error_kind(&err) != *expected_error {
                    report.fail_case(format!(
                        "expected error {expected_error}, got {}",
                        error_kind(&err)
                    ));
                }
            }
            (expected, got) => {
                report.fail_case(format!("expected {expected:?}, got {got:?}"));
            }
        }
        // ReflectRef routing sanity for the same subjects (domain enum used here)
        match subject.reflect_ref() {
            ReflectRef::Indirection(_) => {
                if !matches!(case.subject, Subject::Boxed(_)) {
                    report.fail_case("plain value routed to Indirection");
                }
            }
            _ => {
                if matches!(case.subject, Subject::Boxed(_)) {
                    report.fail_case("boxed value did not route to Indirection");
                }
            }
        }
    }
    report.finish();
}
