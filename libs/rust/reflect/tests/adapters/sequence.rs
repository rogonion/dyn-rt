//! Adapter suite for `src/adapters/sequence.rs`: ReflectSequence on Value
//! arrays and Vec<T>.

use crate::common::{arr, error_kind, obj, value_of};
use dyn_rt_reflect::{Reflect, ReflectError, ReflectRefMut, as_sequence};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

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

/// Run a mutating sequence op, returning Ok(()) on success or the error kind.
fn run_mutating(value: &mut Value, op: &SeqOp) -> Result<(), &'static str> {
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
    result.map_err(|err| error_kind(&err))
}

#[test]
fn value_sequence_ops() {
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
            } => match run_mutating(&mut value, &case.op) {
                Ok(()) => {
                    if Some(&value) != expected.as_ref() {
                        failures.push(format!(
                            "{title} || set [{index}]: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Err(got) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => failures.push(format!(
                        "{title} || set [{index}]: expected error {expect_error:?}, got {got:?}"
                    )),
                },
            },
            SeqOp::Push {
                expected,
                expect_error,
                ..
            } => match run_mutating(&mut value, &case.op) {
                Ok(()) => {
                    if Some(&value) != expected.as_ref() {
                        failures.push(format!(
                            "{title} || push: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Err(got) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => failures.push(format!(
                        "{title} || push: expected error {expect_error:?}, got {got:?}"
                    )),
                },
            },
            SeqOp::Delete {
                index,
                expected,
                expect_error,
            } => match run_mutating(&mut value, &case.op) {
                Ok(()) => {
                    if Some(&value) != expected.as_ref() {
                        failures.push(format!(
                            "{title} || delete [{index}]: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Err(got) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => failures.push(format!(
                        "{title} || delete [{index}]: expected error {expect_error:?}, got {got:?}"
                    )),
                },
            },
        }
    }
    report_failures(failures, test_data.len());
}
