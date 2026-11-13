//! Adapter suite for `src/adapters/indirection.rs`: Box<T> inner access.

use crate::common::{error_kind, obj, value_of};
use dyn_rt_reflect::{Reflect, ReflectKind, ReflectRefMut, kind_of, reflect_with_inner};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

enum IndOp {
    Kind(&'static str),
    Inner(Value),
    SetInner { set: Value, expected: Value },
    DeleteInner { expect_error: &'static str },
}

struct IndCase {
    test_title: Option<String>,
    inner: Value,
    op: IndOp,
}

macro_rules! ind_case {
    ($inner:expr, $op:expr $(, test_title = $title:expr)?) => {
        IndCase {
test_title: None $(.or(Some($title.to_string())))?,
            inner: $inner,
            op: $op,
        }
    };
}

#[test]
fn box_indirection_ops() {
    let test_data: Vec<_> = vec![
        ind_case!(
            obj(vec![("a", 1i64.into())]),
            IndOp::Inner(obj(vec![("a", 1i64.into())])),
            test_title = "box inner read"
        ),
        ind_case!(
            obj(vec![("a", 1i64.into())]),
            IndOp::SetInner {
                set: obj(vec![("b", 2i64.into())]),
                expected: obj(vec![("b", 2i64.into())])
            },
            test_title = "box set_inner replaces"
        ),
        ind_case!(
            1i64.into(),
            IndOp::DeleteInner {
                expect_error: "UnsupportedOperation"
            },
            test_title = "box delete_inner unsupported"
        ),
        ind_case!(
            1i64.into(),
            IndOp::Kind("indirection"),
            test_title = "box kind is indirection"
        ),
    ];

    let mut report = TestReport::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.test_title);
        report.begin_case(title.clone());
        let mut subject: Box<dyn Reflect> = Box::new(Box::new(case.inner.clone()));

        match &case.op {
            IndOp::Kind(expected) => {
                if kind_of(subject.as_ref()).to_string() != *expected {
                    report.fail_case(format!(
                        "kind: expected {expected:?}, got {:?}",
                        kind_of(subject.as_ref())
                    ));
                }
                if kind_of(subject.as_ref()) != ReflectKind::Indirection {
                    report.fail_case("kind enum: expected Indirection");
                }
            }
            IndOp::Inner(expected) => {
                match reflect_with_inner(subject.as_ref(), |v| value_of(v).clone()) {
                    Ok(inner) if &inner == expected => {}
                    Ok(inner) => {
                        report.fail_case(format!("inner: expected {expected:?}, got {inner:?}"))
                    }
                    Err(err) => {
                        report.fail_case(format!("inner: unexpected error {}", error_kind(&err)))
                    }
                }
            }
            IndOp::SetInner { set, expected } => match subject.reflect_ref_mut() {
                ReflectRefMut::Indirection(ind) => match ind
                    .reflect_set_inner(Box::new(set.clone()))
                {
                    Ok(()) => match reflect_with_inner(subject.as_ref(), |v| value_of(v).clone()) {
                        Ok(inner) if &inner == expected => {}
                        Ok(inner) => report
                            .fail_case(format!("set_inner: expected {expected:?}, got {inner:?}")),
                        Err(err) => {
                            report.fail_case(format!("set_inner read-back: {}", error_kind(&err)))
                        }
                    },
                    Err(err) => report
                        .fail_case(format!("set_inner: unexpected error {}", error_kind(&err))),
                },
                _ => report.fail_case("set_inner: expected indirection"),
            },
            IndOp::DeleteInner { expect_error } => {
                let result = match subject.reflect_ref_mut() {
                    ReflectRefMut::Indirection(ind) => ind.reflect_delete_inner(),
                    _ => {
                        report.fail_case("delete_inner: expected indirection");
                        continue;
                    }
                };
                match &result {
                    Err(err) if error_kind(err) == *expect_error => {}
                    other => report.fail_case(format!(
                        "delete_inner: expected error {expect_error}, got {other:?}"
                    )),
                }
            }
        }
    }
    report.finish();
}
