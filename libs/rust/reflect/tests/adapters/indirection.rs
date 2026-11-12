//! Adapter suite for `src/adapters/indirection.rs`: Box<T> inner access.

use crate::common::{error_kind, obj, value_of};
use dyn_rt_reflect::{Reflect, ReflectKind, ReflectRefMut, kind_of, reflect_with_inner};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

enum IndOp {
    Kind(&'static str),
    Inner(Value),
    SetInner { set: Value, expected: Value },
    DeleteInner { expect_error: &'static str },
}

struct IndCase {
    meta: TestMetadata,
    inner: Value,
    op: IndOp,
}

macro_rules! ind_case {
    ($inner:expr, $op:expr $(, test_title = $title:expr)?) => {
        IndCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
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

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut subject: Box<dyn Reflect> = Box::new(Box::new(case.inner.clone()));

        match &case.op {
            IndOp::Kind(expected) => {
                if kind_of(subject.as_ref()).to_string() != *expected {
                    failures.push(format!(
                        "{title} || kind: expected {expected:?}, got {:?}",
                        kind_of(subject.as_ref())
                    ));
                }
                if kind_of(subject.as_ref()) != ReflectKind::Indirection {
                    failures.push(format!("{title} || kind enum: expected Indirection"));
                }
            }
            IndOp::Inner(expected) => {
                match reflect_with_inner(subject.as_ref(), |v| value_of(v).clone()) {
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
                    Ok(()) => match reflect_with_inner(subject.as_ref(), |v| value_of(v).clone()) {
                        Ok(inner) if &inner == expected => {}
                        Ok(inner) => failures.push(format!(
                            "{title} || set_inner: expected {expected:?}, got {inner:?}"
                        )),
                        Err(err) => failures.push(format!(
                            "{title} || set_inner read-back: {}",
                            error_kind(&err)
                        )),
                    },
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
        }
    }
    report_failures(failures, test_data.len());
}
