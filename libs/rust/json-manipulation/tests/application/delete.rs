//! Data-driven suite for src/application/delete.rs (ADR 0004/0009): cases
//! ported from go-json's object/delete_test.go.

use crate::common::*;
use dyn_rt_json_manipulation::application::delete;
use dyn_rt_json_path::{RegexPathTokenizer, parse_path};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

struct DeleteCase {
    test_title: Option<String>,
    root: Box<dyn Reflect>,
    path: &'static str,
    expected_count: usize,
    /// rendered root after the delete
    expected_root: String,
    /// how to render the root for comparison (Value/scalar default)
    render_root: fn(&dyn Reflect) -> String,
    expect_error: Option<&'static str>,
}

macro_rules! delete_case {
    ($root:expr, $path:expr, $count:expr, $expected_root:expr $(, expect_error = $err:expr)? $(, render = $render:expr)? $(, test_title = $title:expr)?) => {
        DeleteCase {
            test_title: None $(.or(Some($title.to_string())))?,
            root: Box::new($root),
            path: $path,
            expected_count: $count,
            expected_root: $expected_root,
            render_root: {
                let r: Option<fn(&dyn Reflect) -> String> = None $(.or(Some($render)))?;
                r.unwrap_or(render)
            },
            expect_error: None $(.or(Some($err)))?,
        }
    };
}

fn rv(value: Value) -> String {
    format!("{value:?}")
}

#[test]
fn delete_cases() {
    let test_data: Vec<DeleteCase> = vec![
        // delete a map entry
        delete_case!(
            Value::object([("address", Value::object([("address", Value::from("test"))]))]),
            "$.address",
            1,
            rv(Value::Object(Vec::new())),
            test_title = "delete map entry (go $.address)"
        ),
        // delete non-existent map key: no-op, count 0
        delete_case!(
            Value::object([("a", Value::from(1i64))]),
            "$.b",
            0,
            rv(Value::object([("a", Value::from(1i64))])),
            test_title = "delete non-existent map key is a no-op (go)"
        ),
        // delete a sequence element (shift left)
        delete_case!(
            Value::from(vec![
                Value::from(1i64),
                Value::from(2i64),
                Value::from(3i64),
                Value::from(4i64)
            ]),
            "$[1]",
            1,
            rv(Value::from(vec![
                Value::from(1i64),
                Value::from(3i64),
                Value::from(4i64)
            ])),
            test_title = "delete slice element shifts left (go $[1])"
        ),
        // delete index out of bounds
        delete_case!(
            Value::from(vec![Value::from(1i64), Value::from(2i64)]),
            "$[5]",
            0,
            rv(Value::from(vec![Value::from(1i64), Value::from(2i64)])),
            expect_error = "out of range",
            test_title = "delete index out of bounds errors (go)"
        ),
        // delete a struct field resets it to zero
        delete_case!(
            ComplexData {
                user: User {
                    id: 10,
                    name: "amy".to_string(),
                    email: "a@b.c".to_string()
                },
                ..Default::default()
            },
            "$.User.Name",
            1,
            format!(
                "{:?}",
                ComplexData {
                    user: User {
                        id: 10,
                        name: String::new(),
                        email: "a@b.c".to_string()
                    },
                    ..Default::default()
                }
            ),
            render = render_debug::<ComplexData>,
            test_title = "delete struct field resets it to zero (go)"
        ),
        // union delete on struct fields
        delete_case!(
            ComplexData {
                user: User {
                    id: 10,
                    name: "amy".to_string(),
                    email: "a@b.c".to_string()
                },
                ..Default::default()
            },
            "$.User['Name','Email']",
            2,
            format!(
                "{:?}",
                ComplexData {
                    user: User {
                        id: 10,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            ),
            render = render_debug::<ComplexData>,
            test_title = "union delete on struct fields (go)"
        ),
        // recursive descent delete
        delete_case!(
            Value::from(vec![
                Value::object([("User", Value::object([("Name", Value::from("Alice"))]))]),
                Value::object([(
                    "Items",
                    Value::from(vec![Value::object([("Name", Value::from("Item 1"))]),])
                )]),
            ]),
            "$..Name",
            2,
            rv(Value::from(vec![
                Value::object([("User", Value::Object(Vec::new()))]),
                Value::object([("Items", Value::from(vec![Value::Object(Vec::new()),]))]),
            ])),
            test_title = "recursive descent delete removes every Name (go $..Name)"
        ),
        // slice selector delete
        delete_case!(
            Value::from(vec![
                Value::from(1i64),
                Value::from(2i64),
                Value::from(3i64),
                Value::from(4i64)
            ]),
            "$[::2]",
            2,
            rv(Value::from(vec![Value::from(2i64), Value::from(4i64)])),
            test_title = "slice selector deletes every second element (go $[::2])"
        ),
        // root delete resets the source to its zero value
        delete_case!(
            Value::object([("a", Value::from(1i64))]),
            "$",
            1,
            rv(Value::Null),
            test_title = "delete root resets source to zero (go Delete root object)"
        ),
        // key on a primitive errors
        delete_case!(
            Value::from(123i64),
            "$.key",
            0,
            rv(Value::from(123i64)),
            expect_error = "unsupported",
            test_title = "delete property from primitive errors (go)"
        ),
    ];

    let mut report = TestReport::new(
        "delete_cases",
        "delete at a JSONPath (src/application/delete.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let mut root = case.root.reflect_clone();
        let segments = parse_path(&RegexPathTokenizer, case.path);
        let result = delete(&mut *root, &segments);
        match (&case.expect_error, result) {
            (Some(fragment), Err(err)) => {
                if !err.to_string().contains(fragment) {
                    report.fail_case(format!("expected error containing {fragment:?}, got {err}"));
                }
            }
            (Some(fragment), Ok(count)) => {
                report.fail_case(format!(
                    "expected error containing {fragment:?}, got Ok({count})"
                ));
            }
            (None, Err(err)) => {
                report.fail_case(format!("unexpected error: {err}"));
            }
            (None, Ok(count)) => {
                if count != case.expected_count {
                    report.fail_case(format!(
                        "path {} || expected count {}, got {}",
                        case.path, case.expected_count, count
                    ));
                }
                let rendered = (case.render_root)(&*root);
                if rendered != case.expected_root {
                    report.fail_case(format!(
                        "path {} || expected root {}, got {}",
                        case.path, case.expected_root, rendered
                    ));
                }
            }
        }
    }
    report.finish();
}
