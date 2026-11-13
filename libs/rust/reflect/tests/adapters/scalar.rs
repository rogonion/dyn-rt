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

    let mut report = TestReport::new(
        "scalar_clone_apply",
        "Adapter suite for `src/adapters/scalar.rs`: scalar clone/apply, and",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
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

// ---------------------------------------------------------------------------
// unit `()` (src/adapters/scalar.rs): scalar shape, renders as "null"
// ---------------------------------------------------------------------------

enum UnitOp {
    KindIsScalar,
    ScalarStringIsNull,
    CloneAndZero,
    ApplyMismatch,
}

struct UnitCase {
    test_title: Option<String>,
    op: UnitOp,
}

macro_rules! unit_case {
    ($op:expr $(, test_title = $title:expr)?) => {
        UnitCase {
            test_title: None $(.or(Some($title.to_string())))?,
            op: $op,
        }
    };
}

#[test]
fn unit_cases() {
    use dyn_rt_reflect::{ReflectKind, ReflectNew};

    let test_data: Vec<_> = vec![
        unit_case!(
            UnitOp::KindIsScalar,
            test_title = "unit routes to the scalar shape"
        ),
        unit_case!(
            UnitOp::ScalarStringIsNull,
            test_title = "unit renders as \"null\" like Value::Null"
        ),
        unit_case!(
            UnitOp::CloneAndZero,
            test_title = "unit clone/zero/new all produce unit"
        ),
        unit_case!(
            UnitOp::ApplyMismatch,
            test_title = "apply of a non-unit onto unit fails with ShapeMismatch"
        ),
    ];

    let mut report = TestReport::new(
        "unit_cases",
        "Adapter suite for `src/adapters/scalar.rs`: the unit type `()`",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let unit = ();
        let value: &dyn Reflect = &unit;

        match &case.op {
            UnitOp::KindIsScalar => {
                if value.reflect_ref().kind() != ReflectKind::Scalar {
                    report.fail_case(format!(
                        "kind: expected Scalar, got {:?}",
                        value.reflect_ref().kind()
                    ));
                }
            }
            UnitOp::ScalarStringIsNull => {
                if value.reflect_scalar_string().as_deref() != Some("null") {
                    report.fail_case(format!(
                        "scalar string: expected \"null\", got {:?}",
                        value.reflect_scalar_string()
                    ));
                }
            }
            UnitOp::CloneAndZero => {
                let cloned = value.reflect_clone();
                if cloned.reflect_as_any().downcast_ref::<()>().is_none() {
                    report.fail_case("clone: not a unit".to_string());
                }
                if value.reflect_zero().is_none() {
                    report.fail_case("zero: expected Some".to_string());
                }
                let new = <() as ReflectNew>::reflect_new();
                if new.reflect_as_any().downcast_ref::<()>().is_none() {
                    report.fail_case("new: not a unit".to_string());
                }
            }
            UnitOp::ApplyMismatch => {
                let source: &dyn Reflect = &"not unit".to_string();
                let mut target = ();
                match target.reflect_apply(source) {
                    Ok(()) => {
                        report.fail_case("apply mismatch: expected error, got Ok".to_string())
                    }
                    Err(err) => {
                        if !format!("{err:?}").contains("ShapeMismatch") {
                            report.fail_case(format!(
                                "apply mismatch: expected ShapeMismatch, got {err:?}"
                            ));
                        }
                    }
                }
            }
        }
    }
    report.finish();
}
