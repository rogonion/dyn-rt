//! Data-driven suite for src/application/set.rs (ADR 0004/0009): cases ported
//! from go-json's object/set_test.go — including schema-driven creation of
//! intermediate structures.

use crate::common::*;
use dyn_rt_manipulation::application::{are_equal, set};
use dyn_rt_path::{RegexPathTokenizer, parse_path};
use dyn_rt_reflect::Reflect;
use dyn_rt_schema::Schema;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

struct SetCase {
    test_title: Option<String>,
    root: Box<dyn Reflect>,
    path: &'static str,
    value: Box<dyn Reflect>,
    schema: Option<Schema>,
    expected_count: usize,
    /// expected root after the set, compared with `are_equal` (deep
    /// comparison, like go's tests)
    expected_root: Box<dyn Reflect>,
    /// how to render the root for failure messages (Value/scalar default)
    render_root: fn(&dyn Reflect) -> String,
    expect_error: Option<&'static str>,
}

macro_rules! set_case {
    ($root:expr, $path:expr, $value:expr, $schema:expr, $count:expr, $expected_root:expr $(, expect_error = $err:expr)? $(, render = $render:expr)? $(, test_title = $title:expr)?) => {
        SetCase {
            test_title: None $(.or(Some($title.to_string())))?,
            root: Box::new($root),
            path: $path,
            value: Box::new($value),
            schema: $schema,
            expected_count: $count,
            expected_root: Box::new($expected_root) as Box<dyn Reflect>,
            render_root: {
                let r: Option<fn(&dyn Reflect) -> String> = None $(.or(Some($render)))?;
                r.unwrap_or(render)
            },
            expect_error: None $(.or(Some($err)))?,
        }
    };
}

/// Identity helper kept so case entries read uniformly (`rv(...)` = "result
/// value"); the macro boxes the expression.
fn rv<T>(value: T) -> T {
    value
}

#[test]
fn set_cases() {
    let test_data: Vec<SetCase> = vec![
        // set an existing map entry, coercing string -> i64 via the value type
        set_case!(
            Value::object([("a", Value::from(1i64))]),
            "$.a",
            Value::from("42"),
            None,
            1,
            // map value type is any (Value), so the string is stored as-is
            rv(Value::object([("a", Value::from("42"))])),
            test_title = "set existing map entry (go Set map entry)"
        ),
        // create a missing map entry on a Value object (value type is any -> pass-through)
        set_case!(
            Value::object([("a", Value::from(1i64))]),
            "$.b",
            Value::from("new"),
            None,
            1,
            rv(Value::object([
                ("a", Value::from(1i64)),
                ("b", Value::from("new"))
            ])),
            test_title = "set creates a missing map entry"
        ),
        // set a sequence index in place
        set_case!(
            Value::from(vec![Value::from(1i64), Value::from(2i64)]),
            "$[0]",
            Value::from(9i64),
            None,
            1,
            rv(Value::from(vec![Value::from(9i64), Value::from(2i64)])),
            test_title = "set sequence index replaces in place"
        ),
        // set at index == length pushes
        set_case!(
            Value::from(vec![Value::from(1i64)]),
            "$[1]",
            Value::from(2i64),
            None,
            1,
            rv(Value::from(vec![Value::from(1i64), Value::from(2i64)])),
            test_title = "set at index == length appends"
        ),
        // set beyond the end expands with defaults (go: "Set slice index with expansion")
        set_case!(
            Value::from(vec![Value::from(0i64)]),
            "$[3]",
            Value::from(10i64),
            None,
            1,
            rv(Value::from(vec![
                Value::from(0i64),
                Value::Null,
                Value::Null,
                Value::from(10i64)
            ])),
            test_title = "set past the end expands with defaults (go expansion)"
        ),
        // set a struct field through its schema
        set_case!(
            UserProfile::default(),
            "$.Name",
            Value::from("amy"),
            Some(Schema::Node(user_profile_schema())),
            1,
            UserProfile {
                name: "amy".to_string(),
                ..Default::default()
            },
            render = render_debug::<UserProfile>,
            test_title = "set struct field via schema (go auto-init struct)"
        ),
        // schema-driven creation of a nested struct through a nilable field
        set_case!(
            UserProfile::default(),
            "$.Address.ZipCode",
            Value::from("1234"),
            Some(Schema::Node(user_profile_schema())),
            1,
            UserProfile {
                address: Address {
                    zip_code: Some("1234".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
            render = render_debug::<UserProfile>,
            test_title =
                "set creates nested struct + Some payload via schema (go implicit creation)"
        ),
        // union set on a struct: both fields set
        set_case!(
            ComplexData::default(),
            "$.User['Name','Email']",
            Value::from("x@y.z"),
            None,
            2,
            ComplexData {
                user: User {
                    name: "x@y.z".to_string(),
                    email: "x@y.z".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
            render = render_debug::<ComplexData>,
            test_title = "union set on struct fields (go $.User['Name','Email'])"
        ),
        // slice selector set
        set_case!(
            Value::from(vec![
                Value::from(1i64),
                Value::from(2i64),
                Value::from(3i64),
                Value::from(4i64)
            ]),
            "$[::2]",
            Value::from(0i64),
            None,
            2,
            rv(Value::from(vec![
                Value::from(0i64),
                Value::from(2i64),
                Value::from(0i64),
                Value::from(4i64)
            ])),
            test_title = "slice selector sets every second element (go $[::2])"
        ),
        // recursive descent set: all Name fields
        set_case!(
            Value::from(vec![
                Value::object([("User", Value::object([("Name", Value::from("Alice"))]))]),
                Value::object([(
                    "Items",
                    Value::from(vec![Value::object([("Name", Value::from("Item 1"))]),])
                )]),
            ]),
            "$..Name",
            Value::from("OneName"),
            None,
            2,
            rv(Value::from(vec![
                Value::object([("User", Value::object([("Name", Value::from("OneName"))]))]),
                Value::object([(
                    "Items",
                    Value::from(vec![Value::object([("Name", Value::from("OneName"))]),])
                )]),
            ])),
            test_title = "recursive descent set updates every Name (go $..Name)"
        ),
        // root set replaces the source
        set_case!(
            Value::from(1i64),
            "$",
            Value::from("root"),
            None,
            1,
            rv(Value::from("root")),
            test_title = "set root replaces source (go Set root object)"
        ),
        // type mismatch errors (go: Set type mismatch): typed map value i64
        set_case!(
            {
                let mut m = std::collections::HashMap::new();
                m.insert("a".to_string(), 1i64);
                m
            },
            "$.a",
            Value::object([("x", Value::from(1i64))]),
            None,
            0,
            String::new(),
            expect_error = "convert",
            test_title = "setting an object into an i64 map entry errors (go type mismatch)"
        ),
        // union with a negative index member sets first and last
        set_case!(
            Value::from(vec![
                Value::from(1i64),
                Value::from(2i64),
                Value::from(3i64)
            ]),
            "$[0,-1]",
            Value::from(99i64),
            None,
            2,
            rv(Value::from(vec![
                Value::from(99i64),
                Value::from(2i64),
                Value::from(99i64)
            ])),
            test_title = "union set with negative index member sets first and last"
        ),
        // union with a slice member sets every index the slice expands to
        set_case!(
            Value::from((0..5).map(|i| Value::from(i as i64)).collect::<Vec<_>>()),
            "$[1, 2:4]",
            Value::from(9i64),
            None,
            3,
            rv(Value::from(vec![
                Value::from(0i64),
                Value::from(9i64),
                Value::from(9i64),
                Value::from(9i64),
                Value::from(4i64)
            ])),
            test_title = "union set with slice member sets the expanded indices"
        ),
        // --- native built-in types (not Value) -----------------------------
        set_case!(
            {
                let mut m = std::collections::HashMap::new();
                m.insert("a".to_string(), 1i64);
                m
            },
            "$.a",
            5i64,
            None,
            1,
            {
                let mut m = std::collections::HashMap::new();
                m.insert("a".to_string(), 5i64);
                m
            },
            render = render_debug::<std::collections::HashMap<String, i64>>,
            test_title = "native HashMap<String, i64> set existing entry"
        ),
        set_case!(
            std::collections::HashMap::<String, i64>::new(),
            "$.new_key",
            7i64,
            None,
            1,
            {
                let mut m = std::collections::HashMap::new();
                m.insert("new_key".to_string(), 7i64);
                m
            },
            render = render_debug::<std::collections::HashMap<String, i64>>,
            test_title = "native HashMap set creates a missing entry"
        ),
        set_case!(
            vec![
                User {
                    id: 1,
                    name: "Ann".to_string(),
                    email: "a@x".to_string()
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "b@x".to_string()
                },
            ],
            "$[0].Name",
            "X".to_string(),
            None,
            1,
            vec![
                User {
                    id: 1,
                    name: "X".to_string(),
                    email: "a@x".to_string()
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "b@x".to_string()
                },
            ],
            render = render_debug::<Vec<User>>,
            test_title = "native Vec<User> set struct field through index"
        ),
        set_case!(
            [10i64, 20, 30],
            "$[1]",
            99i64,
            None,
            1,
            [10i64, 99, 30],
            render = render_debug::<[i64; 3]>,
            test_title = "fixed-size array [i64; 3] in-place index set"
        ),
        set_case!(
            (1i64, "x".to_string(), true),
            "$.1",
            "y".to_string(),
            None,
            1,
            (1i64, "y".to_string(), true),
            render = render_debug::<(i64, String, bool)>,
            test_title = "tuple positional field set"
        ),
        set_case!(
            UserWithAddress::default(),
            "$.Address.City",
            "Mombasa".to_string(),
            None,
            1,
            UserWithAddress {
                name: String::new(),
                address: Box::new(Address {
                    street: String::new(),
                    city: "Mombasa".to_string(),
                    zip_code: None,
                }),
            },
            render = render_debug::<UserWithAddress>,
            test_title = "set through Box<Address> into a defaulted struct (smart pointer)"
        ),
    ];

    let mut report = TestReport::new(
        "set_cases",
        "set at a JSONPath with schema-driven coercion/creation (src/application/set.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let mut root = case.root.reflect_clone();
        let segments = parse_path(&RegexPathTokenizer, case.path);
        let result = set(&mut *root, &segments, &*case.value, case.schema.as_ref());
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
                if !are_equal(&*case.expected_root, &*root) {
                    report.fail_case(format!(
                        "path {} || expected root {}, got {}",
                        case.path,
                        (case.render_root)(&*case.expected_root),
                        (case.render_root)(&*root)
                    ));
                }
            }
        }
    }
    report.finish();
}
