//! Adapter suite for `src/adapters/scalar.rs`: scalar clone/apply, and
//! clone/apply on Value via the base Reflect impl (adapters/value.rs).

use crate::common::{arr, error_kind, obj, value_of};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

enum CloneApplyOp {
    Clone {
        expected: Value,
    },
    Apply {
        source: Value,
        expected: Value,
    },
    ApplyMismatch {
        source: &'static str,
        expect_error: &'static str,
    },
}

struct ScalarCase {
    test_title: Option<String>,
    value: Value,
    op: CloneApplyOp,
}

macro_rules! scalar_case {
    ($value:expr, $op:expr $(, test_title = $title:expr)?) => {
        ScalarCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            op: $op,
        }
    };
}

#[test]
fn scalar_clone_apply() {
    let test_data: Vec<_> = vec![
        scalar_case!(
            obj(vec![("a", arr(vec![1i64.into(), 2i64.into()]))]),
            CloneApplyOp::Clone {
                expected: obj(vec![("a", arr(vec![1i64.into(), 2i64.into()]))])
            },
            test_title = "clone of object is deep and equal"
        ),
        scalar_case!(
            Value::from(5i64),
            CloneApplyOp::Clone {
                expected: 5i64.into()
            },
            test_title = "clone of scalar"
        ),
        scalar_case!(
            arr(vec![1i64.into(), "x".into()]),
            CloneApplyOp::Clone {
                expected: arr(vec![1i64.into(), "x".into()])
            },
            test_title = "clone of array"
        ),
        scalar_case!(
            obj(vec![("a", 1i64.into())]),
            CloneApplyOp::Apply {
                source: obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
                expected: obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
            },
            test_title = "apply copies structurally"
        ),
        scalar_case!(
            obj(vec![("a", 1i64.into())]),
            CloneApplyOp::ApplyMismatch {
                source: "hello",
                expect_error: "ShapeMismatch"
            },
            test_title = "apply with wrong concrete type fails"
        ),
    ];

    let mut report = TestReport::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.test_title);
        report.begin_case(title.clone());
        let mut value = case.value.clone();

        match &case.op {
            CloneApplyOp::Clone { expected } => {
                let reflected: &dyn Reflect = &value;
                let cloned = reflected.reflect_clone();
                let got = value_of(cloned.as_ref());
                if got != expected {
                    report.fail_case(format!("clone: expected {expected:?}, got {got:?}"));
                }
            }
            CloneApplyOp::Apply { source, expected } => {
                let source_ref: &dyn Reflect = source;
                match value.reflect_apply(source_ref) {
                    Ok(()) => {
                        if value != *expected {
                            report
                                .fail_case(format!("apply: expected {expected:?}, got {value:?}"));
                        }
                    }
                    Err(err) => {
                        report.fail_case(format!("apply: unexpected error {}", error_kind(&err)))
                    }
                }
            }
            CloneApplyOp::ApplyMismatch {
                source,
                expect_error,
            } => {
                let source_string = source.to_string();
                let source_ref: &dyn Reflect = &source_string;
                match value.reflect_apply(source_ref) {
                    Err(err) if error_kind(&err) == *expect_error => {}
                    other => report.fail_case(format!(
                        "apply mismatch: expected error {expect_error}, got {other:?}"
                    )),
                }
            }
        }
    }
    report.finish();
}
