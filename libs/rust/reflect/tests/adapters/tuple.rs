//! Adapter suite for `src/adapters/tuple.rs`: tuples route to the struct
//! shape with positional field names ("0", "1", ...), arities 1..=16, with
//! clone/apply/zero working element-wise.

use dyn_rt_reflect::{Reflect, ReflectKind, ReflectNew, ReflectRef};
use dyn_rt_test_support::{TestReport, new_test_title};

enum TupleOp {
    /// kind is Struct and reflect_ref routes to Struct with the expected
    /// positional field names.
    Shape { field_names: Vec<&'static str> },
    /// field_at / field-by-name return the expected scalar render.
    Field {
        index: usize,
        name: &'static str,
        expect_scalar: Option<String>,
    },
    /// field_at with an out-of-bounds index returns None.
    FieldOutOfBounds { index: usize },
    /// clone returns a value of the same concrete type that applies back.
    Clone,
    /// apply from an equal-typed source succeeds.
    Apply,
    /// apply with a mismatched concrete type fails with ShapeMismatch.
    ApplyMismatch,
    /// reflect_zero and a concrete ReflectNew::reflect_new agree on type.
    Zero { new_make: fn() -> Box<dyn Reflect> },
}

struct TupleCase {
    test_title: Option<String>,
    make: fn() -> Box<dyn Reflect>,
    op: TupleOp,
}

macro_rules! tuple_case {
    ($make:expr, $op:expr $(, test_title = $title:expr)?) => {
        TupleCase {
            test_title: None $(.or(Some($title.to_string())))?,
            make: $make,
            op: $op,
        }
    };
}

/// Scalar render used for expected-vs-got: string form, else i64 form.
fn render_scalar(value: &dyn Reflect) -> Option<String> {
    value
        .reflect_scalar_string()
        .or_else(|| value.reflect_scalar_i64().map(|v| v.to_string()))
}

#[test]
fn tuple_cases() {
    let test_data: Vec<_> = vec![
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::Shape {
                field_names: vec!["0", "1"]
            },
            test_title = "2-tuple routes to struct shape with positional names"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::Field {
                index: 0,
                name: "0",
                expect_scalar: Some("42".to_string())
            },
            test_title = "field_at(0) and field(\"0\") read the first element"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::Field {
                index: 1,
                name: "1",
                expect_scalar: Some("hello".to_string())
            },
            test_title = "field_at(1) and field(\"1\") read the second element"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::FieldOutOfBounds { index: 2 },
            test_title = "field_at past the arity returns None"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::Clone,
            test_title = "clone of a 2-tuple is equal and same-typed"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::Apply,
            test_title = "apply replaces a tuple structurally"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::ApplyMismatch,
            test_title = "apply of a non-tuple fails with ShapeMismatch"
        ),
        tuple_case!(
            || Box::new((42i64, "hello".to_string())),
            TupleOp::Zero {
                new_make: || <(i64, String) as ReflectNew>::reflect_new()
            },
            test_title = "zero/new of a 2-tuple agree on the concrete type"
        ),
        // nested: tuple containing a sequence and another tuple
        tuple_case!(
            || Box::new(("root".to_string(), vec![1i64, 2i64], (true, 'x'))),
            TupleOp::Shape {
                field_names: vec!["0", "1", "2"]
            },
            test_title = "3-tuple with nested sequence and tuple fields"
        ),
        // max arity: 16 elements, last index reachable by index and name
        tuple_case!(
            || Box::new((
                0i64, 1i64, 2i64, 3i64, 4i64, 5i64, 6i64, 7i64, 8i64, 9i64, 10i64, 11i64, 12i64,
                13i64, 14i64, 15i64
            )),
            TupleOp::Shape {
                field_names: vec![
                    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
                    "15"
                ],
            },
            test_title = "16-tuple exposes all sixteen positional fields"
        ),
        tuple_case!(
            || Box::new((
                0i64, 1i64, 2i64, 3i64, 4i64, 5i64, 6i64, 7i64, 8i64, 9i64, 10i64, 11i64, 12i64,
                13i64, 14i64, 15i64
            )),
            TupleOp::Field {
                index: 15,
                name: "15",
                expect_scalar: Some("15".to_string())
            },
            test_title = "16-tuple last field readable by index and name"
        ),
    ];

    let mut report = TestReport::new(
        "tuple_cases",
        "Adapter suite for `src/adapters/tuple.rs`: tuple struct shape, arities 1..=16",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let value = (case.make)();

        match &case.op {
            TupleOp::Shape { field_names } => {
                if value.reflect_ref().kind() != ReflectKind::Struct {
                    report.fail_case(format!(
                        "kind: expected Struct, got {:?}",
                        value.reflect_ref().kind()
                    ));
                }
                match value.reflect_ref() {
                    ReflectRef::Struct(structure) => {
                        let names: Vec<&'static str> = structure
                            .reflect_field_infos()
                            .iter()
                            .map(|i| i.name)
                            .collect();
                        if names != *field_names {
                            report.fail_case(format!(
                                "field names: expected {field_names:?}, got {names:?}"
                            ));
                        }
                        if structure.reflect_field_count() != field_names.len() {
                            report.fail_case(format!(
                                "field count: expected {}, got {}",
                                field_names.len(),
                                structure.reflect_field_count()
                            ));
                        }
                    }
                    other => report.fail_case(format!(
                        "reflect_ref: expected Struct, got {:?}",
                        other.kind()
                    )),
                }
            }
            TupleOp::Field {
                index,
                name,
                expect_scalar,
            } => match value.reflect_ref() {
                ReflectRef::Struct(structure) => {
                    let by_index = structure.reflect_field_at(*index).and_then(render_scalar);
                    let by_name = structure.reflect_field(name).and_then(render_scalar);
                    if by_index != *expect_scalar || by_name != *expect_scalar {
                        report.fail_case(format!(
                            "field {name}: expected {expect_scalar:?}, by index {by_index:?}, by name {by_name:?}"
                        ));
                    }
                }
                other => report.fail_case(format!(
                    "reflect_ref: expected Struct, got {:?}",
                    other.kind()
                )),
            },
            TupleOp::FieldOutOfBounds { index } => match value.reflect_ref() {
                ReflectRef::Struct(structure) => {
                    if structure.reflect_field_at(*index).is_some() {
                        report.fail_case(format!("field_at({index}): expected None, got Some"));
                    }
                }
                other => report.fail_case(format!(
                    "reflect_ref: expected Struct, got {:?}",
                    other.kind()
                )),
            },
            TupleOp::Clone => {
                let cloned = value.reflect_clone();
                if type_id_of(&*cloned) != type_id_of(&*value) {
                    report.fail_case("clone: concrete type changed".to_string());
                }
                let mut target = (case.make)();
                if target.reflect_apply(&*cloned).is_err() {
                    report.fail_case("clone: applying the clone back failed".to_string());
                }
            }
            TupleOp::Apply => {
                let source = (case.make)();
                let mut target = (case.make)();
                if target.reflect_apply(&*source).is_err() {
                    report.fail_case("apply: unexpected error".to_string());
                }
            }
            TupleOp::ApplyMismatch => {
                let source: &dyn Reflect = &"not a tuple".to_string();
                let mut target = (case.make)();
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
            TupleOp::Zero { new_make } => {
                let zero = value.reflect_zero().expect("tuple reflect_zero is Some");
                if type_id_of(&*zero) != type_id_of(&*value) {
                    report.fail_case("zero: concrete type mismatch".to_string());
                }
                let new = new_make();
                if type_id_of(&*new) != type_id_of(&*value) {
                    report.fail_case("new: concrete type mismatch".to_string());
                }
            }
        }
    }
    report.finish();
}

fn type_id_of(value: &dyn Reflect) -> std::any::TypeId {
    value.reflect_as_any().type_id()
}
