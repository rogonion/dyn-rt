//! Adapter suite for `src/adapters/sequence.rs`: ReflectSequence on Value
//! arrays and Vec<T>.

use crate::common::{arr, error_kind, obj, value_of};
use dyn_rt_reflect::{Reflect, ReflectError, ReflectRefMut, as_sequence};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

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
    test_title: Option<String>,
    value: Value,
    op: SeqOp,
}

macro_rules! seq_case {
    ($value:expr, $op:expr $(, test_title = $title:expr)?) => {
        SeqCase {
test_title: None $(.or(Some($title.to_string())))?,
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

    let mut report = TestReport::new(
        "value_sequence_ops",
        "Adapter suite for `src/adapters/sequence.rs`: ReflectSequence on Value",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let mut value = case.value.clone();
        let reflected: &dyn Reflect = &value;

        match &case.op {
            SeqOp::Get { index, expected } => {
                let result = as_sequence(reflected).and_then(|s| s.reflect_get(*index));
                let got = result.map(value_of);
                if got != expected.as_ref() {
                    report.fail_case(format!("get [{index}]: expected {expected:?}, got {got:?}"));
                }
            }
            SeqOp::Len(expected) => {
                let seq = as_sequence(reflected).expect("sequence expected");
                if seq.reflect_length() != *expected {
                    report.fail_case(format!(
                        "len: expected {expected}, got {}",
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
                        report.fail_case(format!(
                            "set [{index}]: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Err(got) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => report.fail_case(format!(
                        "set [{index}]: expected error {expect_error:?}, got {got:?}"
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
                        report.fail_case(format!("push: expected {expected:?}, got {value:?}"));
                    }
                }
                Err(got) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => report.fail_case(format!(
                        "push: expected error {expect_error:?}, got {got:?}"
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
                        report.fail_case(format!(
                            "delete [{index}]: expected {expected:?}, got {value:?}"
                        ));
                    }
                }
                Err(got) => match expect_error {
                    Some(expected_error) if got == *expected_error => {}
                    _ => report.fail_case(format!(
                        "delete [{index}]: expected error {expect_error:?}, got {got:?}"
                    )),
                },
            },
        }
    }
    report.finish();
}

// ---------------------------------------------------------------------------
// fixed-size arrays [T; N] (src/adapters/sequence.rs): sequence shape,
// in-place set, push/delete unsupported
// ---------------------------------------------------------------------------

#[test]
fn array_cases() {
    use dyn_rt_reflect::{ReflectKind, ReflectNew};

    let mut report = TestReport::new(
        "array_cases",
        "Adapter suite for `src/adapters/sequence.rs`: fixed-size arrays [T; N]",
    );

    let case_title = "array [i64; 3]: sequence shape, get/set/len, push+delete unsupported";
    report.begin_case(case_title.to_string());
    let mut array = [10i64, 20, 30];

    if Reflect::reflect_ref(&array).kind() != ReflectKind::Sequence {
        report.fail_case(format!(
            "kind: expected Sequence, got {:?}",
            Reflect::reflect_ref(&array).kind()
        ));
    }
    {
        let seq = dyn_rt_reflect::as_sequence(&array).expect("sequence");
        if seq.reflect_length() != 3 {
            report.fail_case(format!("len: expected 3, got {}", seq.reflect_length()));
        }
        match seq.reflect_get(1).and_then(|v| v.reflect_scalar_i64()) {
            Some(20) => {}
            other => report.fail_case(format!("get(1): expected 20, got {other:?}")),
        }
        if seq.reflect_get(3).is_some() {
            report.fail_case("get(3): expected None".to_string());
        }
    }
    {
        let seq = dyn_rt_reflect::as_sequence_mut(&mut array).expect("sequence");
        if seq.reflect_set_index(2, Box::new(99i64)).is_err() {
            report.fail_case("set_index(2): unexpected error".to_string());
        }
        if seq.reflect_set_index(3, Box::new(0i64)).is_ok() {
            report.fail_case("set_index(3): expected IndexOutOfBounds".to_string());
        }
        if seq.reflect_push(Box::new(0i64)).is_ok() {
            report.fail_case("push: expected unsupported error".to_string());
        }
        if seq.reflect_delete_index(0).is_ok() {
            report.fail_case("delete_index: expected unsupported error".to_string());
        }
    }
    if array != [10, 20, 99] {
        report.fail_case(format!(
            "after set_index(2)=99: expected [10, 20, 99], got {array:?}"
        ));
    }
    let zero = array.reflect_zero().expect("array reflect_zero is Some");
    if zero.reflect_as_any().downcast_ref::<[i64; 3]>() != Some(&[0, 0, 0]) {
        report.fail_case("zero: expected [0, 0, 0]".to_string());
    }
    let new = <[i64; 3] as ReflectNew>::reflect_new();
    if new.reflect_as_any().downcast_ref::<[i64; 3]>() != Some(&[0, 0, 0]) {
        report.fail_case("new: expected [0, 0, 0]".to_string());
    }
    let cloned = array.reflect_clone();
    if cloned.reflect_as_any().downcast_ref::<[i64; 3]>() != Some(&[10, 20, 99]) {
        report.fail_case("clone: expected [10, 20, 99]".to_string());
    }

    report.finish();
}
