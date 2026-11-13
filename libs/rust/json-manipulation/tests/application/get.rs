//! Data-driven suite for src/application/get.rs (ADR 0004/0009): cases ported
//! from go-json's object/get_test.go, expressed over Value trees and the
//! shared struct fixtures.

use crate::common::*;
use dyn_rt_json_manipulation::application::get;
use dyn_rt_json_path::{RegexPathTokenizer, parse_path};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

struct GetCase {
    test_title: Option<String>,
    root: Box<dyn Reflect>,
    path: &'static str,
    /// expected rendered results (empty = path matches nothing)
    expected_values: Vec<String>,
    /// when set, get must fail with a message containing this fragment
    expect_error: Option<&'static str>,
}

macro_rules! get_case {
    ($root:expr, $path:expr, [$($expected:expr),*] $(, expect_error = $err:expr)? $(, test_title = $title:expr)?) => {
        GetCase {
            test_title: None $(.or(Some($title.to_string())))?,
            root: Box::new($root),
            path: $path,
            expected_values: vec![$($expected.to_string()),*],
            expect_error: None $(.or(Some($err)))?,
        }
    };
}

/// Shorthand for a Value in rendered form.
fn rv(value: Value) -> String {
    format!("{value:?}")
}

#[test]
fn get_cases() {
    let test_data: Vec<GetCase> = vec![
        // go case 1: complex nested path with maps and structs
        get_case!(
            Value::object([(
                "data",
                Value::object([("metadata", Value::from("placeholder"),)])
            ),]),
            "$.data.metadata",
            [rv(Value::from("placeholder"))],
            test_title = "nested map path"
        ),
        // nested struct field via fixtures (go's anonymous struct case)
        get_case!(
            UserProfile {
                name: "amy".to_string(),
                age: 30,
                address: Address {
                    street: "123 Main St".to_string(),
                    city: "Anytown".to_string(),
                    zip_code: None,
                },
            },
            "$.Address.City",
            [render(&"Anytown".to_string())],
            test_title = "struct field path (go case 1 equivalent)"
        ),
        // go case 2: recursive descent with wildcard-ish key over mixed slice
        get_case!(
            Value::from(vec![
                Value::object([("User", Value::object([("Name", Value::from("Alice"))]))]),
                Value::object([("User", Value::object([("Name", Value::from("Bob"))]))]),
                Value::object([(
                    "Items",
                    Value::from(vec![
                        Value::object([("Name", Value::from("Item 1"))]),
                        Value::object([("Name", Value::from("Item 2"))]),
                    ])
                )]),
            ]),
            "$..Name",
            [
                rv(Value::from("Alice")),
                rv(Value::from("Bob")),
                rv(Value::from("Item 1")),
                rv(Value::from("Item 2"))
            ],
            test_title = "recursive descent collects all Name fields (go case 2)"
        ),
        // go case 3: union on a struct
        get_case!(
            ComplexData {
                user: User {
                    id: 10,
                    name: "Charlie".to_string(),
                    email: "charlie@example.com".to_string()
                },
                ..Default::default()
            },
            "$.User['ID', 'Name']",
            [render(&10i64), render(&"Charlie".to_string())],
            test_title = "union operator over struct fields (go case 3)"
        ),
        // go case 4: deep path with index and field
        get_case!(
            ComplexData {
                items: vec![
                    ComplexDataItem {
                        name: "First".to_string(),
                        value: 10
                    },
                    ComplexDataItem {
                        name: "Second".to_string(),
                        value: 20
                    },
                    ComplexDataItem {
                        name: "Third".to_string(),
                        value: 30
                    },
                ],
                ..Default::default()
            },
            "$.Items[1].Value",
            [render(&20i64)],
            test_title = "slice index then struct field (go case 4)"
        ),
        // go case 5: path that matches nothing
        get_case!(
            Value::object([("store", Value::object([("book", Value::from("fiction"))]))]),
            "$.store.magazine",
            [],
            test_title = "missing map key yields no results (go case 5)"
        ),
        // go: recursive descent + slice + union
        get_case!(
            Value::from(vec![
                Value::object([(
                    "one",
                    Value::object([(
                        "Three",
                        Value::from(vec![
                            Value::from(0i64),
                            Value::from(1i64),
                            Value::from(2i64),
                            Value::from(3i64),
                            Value::object([(
                                "04",
                                Value::from(vec![Value::from(0i64), Value::from(4i64)])
                            )]),
                            Value::from(5i64),
                        ])
                    )])
                )]),
                Value::object([(
                    "Two",
                    Value::object([(
                        "Three",
                        Value::from(vec![
                            Value::from(0i64),
                            Value::from(3i64),
                            Value::object([("TwentyFour", Value::from(24i64))]),
                        ])
                    )])
                )]),
            ]),
            "$..Three[::2]['TwentyFour','04']",
            [
                rv(Value::from(vec![Value::from(0i64), Value::from(4i64)])),
                rv(Value::from(24i64))
            ],
            test_title = "recursive descent with slice then union (go case 6 equivalent)"
        ),
        // go: wildcard on map values found via recursive descent (go $.one..Two[*])
        get_case!(
            Value::object([(
                "one",
                Value::from(vec![
                    Value::from(1i64),
                    Value::object([("Two", Value::from(vec![Value::from(12i64)]))]),
                    Value::object([("three", Value::from(vec![Value::from("four")]))]),
                    Value::object([(
                        "five",
                        Value::from(vec![Value::object([(
                            "Two",
                            Value::from(vec![Value::from(13i64)])
                        )]),])
                    )]),
                ])
            )]),
            "$.one..Two[*]",
            [rv(Value::from(12i64)), rv(Value::from(13i64))],
            test_title = "descent into key then wildcard over map values (go $.one..Two[*])"
        ),
        // go: index into map-valued slice element by key
        get_case!(
            Value::object([(
                "three",
                Value::from(vec![
                    Value::object([("0", Value::from(0i64))]),
                    Value::object([("1", Value::from(1i64))]),
                ])
            )]),
            "$.three[1].['1']",
            [rv(Value::from(1i64))],
            test_title = "index then bracket key (go $.three[1].['1'])"
        ),
        // go: root
        get_case!(
            Value::from("root value"),
            "$",
            [rv(Value::from("root value"))],
            test_title = "get root object (go)"
        ),
        // go: index out of bounds
        get_case!(
            Value::from(vec![Value::from(1i64), Value::from(2i64)]),
            "$[5]",
            [],
            expect_error = "out of range",
            test_title = "index out of bounds errors (go)"
        ),
        // go: property from primitive
        get_case!(
            Value::from(123i64),
            "$.key",
            [],
            expect_error = "unsupported",
            test_title = "key on primitive errors (go)"
        ),
        // int map keys: an index segment on a map is unsupported (go errors too)
        get_case!(
            {
                let mut m = std::collections::HashMap::new();
                m.insert(7i64, Value::from("seven"));
                m
            },
            "$[7]",
            [],
            expect_error = "unsupported",
            test_title = "index segment on an int-keyed map errors"
        ),
        // map with int keys addressed by bracket key string: key converts
        get_case!(
            {
                let mut m = std::collections::HashMap::new();
                m.insert(3i64, Value::from("three"));
                m
            },
            "$['3']",
            [rv(Value::from("three"))],
            test_title = "string path key coerced to int map key (go map key conversion)"
        ),
        // go: map key conversion failure (map[int] addressed by non-numeric key)
        get_case!(
            {
                let mut m: std::collections::HashMap<i64, String> =
                    std::collections::HashMap::new();
                m.insert(1i64, "one".to_string());
                m
            },
            "$.not_an_int",
            [],
            expect_error = "conversion failed",
            test_title = "non-numeric key on int-keyed map fails conversion (go)"
        ),
        // slice selector on a sequence
        get_case!(
            Value::from((0..6).map(|i| Value::from(i as i64)).collect::<Vec<_>>()),
            "$[1:5:2]",
            [rv(Value::from(1i64)), rv(Value::from(3i64))],
            test_title = "slice selector [1:5:2]"
        ),
        // wildcard terminal on a sequence
        get_case!(
            Value::from(vec![
                Value::from(1i64),
                Value::from(2i64),
                Value::from(3i64)
            ]),
            "$[*]",
            [
                rv(Value::from(1i64)),
                rv(Value::from(2i64)),
                rv(Value::from(3i64))
            ],
            test_title = "wildcard over sequence"
        ),
        // union of indices
        get_case!(
            Value::from(vec![
                Value::from(10i64),
                Value::from(20i64),
                Value::from(30i64)
            ]),
            "$[0,2]",
            [rv(Value::from(10i64)), rv(Value::from(30i64))],
            test_title = "union of indices"
        ),
        // negative index
        get_case!(
            Value::from(vec![
                Value::from(10i64),
                Value::from(20i64),
                Value::from(30i64)
            ]),
            "$[-1]",
            [rv(Value::from(30i64))],
            test_title = "negative index counts from the end"
        ),
        // Box indirection unwrapping
        get_case!(
            Box::new(Value::object([("a", Value::from(1i64))])),
            "$.a",
            [rv(Value::from(1i64))],
            test_title = "boxed map unwraps through indirection"
        ),
        // Option Some unwrapping
        get_case!(
            Some(Value::from(42i64)),
            "$",
            [rv(Value::from(42i64))],
            test_title = "Some payload is the root value"
        ),
        // union with a negative index member
        get_case!(
            Value::from(vec![
                Value::from(10i64),
                Value::from(20i64),
                Value::from(30i64)
            ]),
            "$[0,-1]",
            [rv(Value::from(10i64)), rv(Value::from(30i64))],
            test_title = "union with negative index member"
        ),
        // union with a slice member
        get_case!(
            Value::from((0..6).map(|i| Value::from(i as i64)).collect::<Vec<_>>()),
            "$[1, 2:4]",
            [
                rv(Value::from(1i64)),
                rv(Value::from(2i64)),
                rv(Value::from(3i64))
            ],
            test_title = "union with a slice member expands the slice"
        ),
        // union with a wildcard member on a map selects all values too
        get_case!(
            Value::object([("a", Value::from(1i64)), ("b", Value::from(2i64)),]),
            "$['a', *]",
            [
                rv(Value::from(1i64)),
                rv(Value::from(1i64)),
                rv(Value::from(2i64))
            ],
            test_title = "union of key and wildcard over a map"
        ),
    ];

    let mut report = TestReport::new(
        "get_cases",
        "get over reflected data at a JSONPath (src/application/get.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let segments = parse_path(&RegexPathTokenizer, case.path);
        let result = get(&*case.root, &segments);
        match (&case.expect_error, result) {
            (Some(fragment), Err(err)) => {
                if !err.to_string().contains(fragment) {
                    report.fail_case(format!("expected error containing {fragment:?}, got {err}"));
                }
            }
            (Some(fragment), Ok(values)) => {
                report.fail_case(format!(
                    "expected error containing {fragment:?}, got {} results",
                    values.len()
                ));
            }
            (None, Err(err)) => {
                report.fail_case(format!("unexpected error: {err}"));
            }
            (None, Ok(values)) => {
                let rendered: Vec<String> = values.iter().map(|v| render(*v)).collect();
                if rendered != case.expected_values {
                    report.fail_case(format!(
                        "path {} || expected {:?}, got {:?}",
                        case.path, case.expected_values, rendered
                    ));
                }
            }
        }
    }
    report.finish();
}
