//! Application suite for `src/application/for_each.rs`: callback per value
//! found at a JSONPath, incl. early stop.

use std::cell::RefCell;

use crate::common::*;
use dyn_rt_manipulation::{are_equal, for_each};
use dyn_rt_path::{RegexPathTokenizer, parse_path};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

struct ForEachCase {
    test_title: Option<String>,
    root: Box<dyn Reflect>,
    path: &'static str,
    /// expected visited values, compared with `are_equal` (deep comparison)
    expected_visited: Vec<Box<dyn Reflect>>,
    /// when set, the callback returns false after this many visits (early stop)
    stop_after: Option<usize>,
}

macro_rules! for_each_case {
    ($root:expr, $path:expr, [$($expected:expr),*] $(, stop_after = $stop:expr)? $(, test_title = $title:expr)?) => {
        ForEachCase {
            test_title: None $(.or(Some($title.to_string())))?,
            root: Box::new($root),
            path: $path,
            expected_visited: vec![$(Box::new($expected) as Box<dyn Reflect>),*],
            stop_after: None $(.or(Some($stop)))?,
        }
    };
}

/// Identity helper kept so case entries read uniformly (`rv(...)` = "result
/// value"); the macro boxes the expression.
fn rv<T>(value: T) -> T {
    value
}

#[test]
fn for_each_cases() {
    let test_data: Vec<ForEachCase> = vec![
        for_each_case!(
            Value::object([
                (
                    "one",
                    Value::from(vec![Value::from(1i64), Value::from(2i64)])
                ),
                ("two", Value::from(3i64)),
            ]),
            "$[*]",
            [
                rv(Value::from(vec![Value::from(1i64), Value::from(2i64)])),
                rv(Value::from(3i64))
            ],
            test_title = "for_each visits every wildcard match"
        ),
        for_each_case!(
            Value::from(vec![
                Value::from(10i64),
                Value::from(20i64),
                Value::from(30i64)
            ]),
            "$[0,-1]",
            [rv(Value::from(10i64)), rv(Value::from(30i64))],
            test_title = "for_each over union with negative index member"
        ),
        for_each_case!(
            Value::object([(
                "items",
                Value::from(vec![
                    Value::object([("Name", Value::from("a"))]),
                    Value::object([("Other", Value::from("x"))]),
                    Value::object([("Name", Value::from("b"))]),
                ])
            )]),
            "$..Name",
            [rv(Value::from("a")), rv(Value::from("b"))],
            test_title = "for_each over recursive descent collects all matches"
        ),
        for_each_case!(
            Value::from(vec![
                Value::from(1i64),
                Value::from(2i64),
                Value::from(3i64)
            ]),
            "$[*]",
            [rv(Value::from(1i64))],
            stop_after = 1,
            test_title = "callback returning false stops iteration early"
        ),
        for_each_case!(
            Value::object([("a", Value::from(1i64))]),
            "$.missing",
            [],
            test_title = "for_each over a non-matching path visits nothing"
        ),
        // native built-ins: recursive descent over a Vec of structs
        for_each_case!(
            vec![
                ComplexDataItem {
                    name: "First".to_string(),
                    value: 1
                },
                ComplexDataItem {
                    name: "Second".to_string(),
                    value: 2
                },
            ],
            "$..Name",
            ["First".to_string(), "Second".to_string()],
            test_title = "for_each recursive descent over native Vec<ComplexDataItem>"
        ),
        // native built-ins: wildcard over a HashMap's values
        for_each_case!(
            {
                let mut m = std::collections::HashMap::new();
                m.insert("a".to_string(), 1i64);
                m
            },
            "$.*",
            [1i64],
            test_title = "for_each wildcard over a native HashMap"
        ),
    ];

    let mut report = TestReport::new(
        "for_each_cases",
        "for_each over reflected data at a JSONPath (src/application/for_each.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let segments = parse_path(&RegexPathTokenizer, case.path);
        let visited: RefCell<Vec<Box<dyn Reflect>>> = RefCell::new(Vec::new());
        let stop_after = case.stop_after;
        let result = for_each(&*case.root, &segments, &mut |value| {
            visited.borrow_mut().push(value.reflect_clone());
            match stop_after {
                Some(limit) => visited.borrow().len() < limit,
                None => true,
            }
        });
        match result {
            Ok(()) => {
                let visited = visited.borrow();
                let mut mismatch = None;
                if visited.len() != case.expected_visited.len() {
                    mismatch = Some(format!(
                        "visited count: expected {}, got {}",
                        case.expected_visited.len(),
                        visited.len()
                    ));
                } else {
                    for (index, (expected, got)) in
                        case.expected_visited.iter().zip(visited.iter()).enumerate()
                    {
                        if !are_equal(&**expected, &**got) {
                            mismatch = Some(format!(
                                "visited {index}: expected {}, got {}",
                                render(&**expected),
                                render(&**got)
                            ));
                            break;
                        }
                    }
                }
                if let Some(detail) = mismatch {
                    report.fail_case(format!("path {} || {}", case.path, detail));
                }
            }
            Err(err) => report.fail_case(format!("unexpected error: {err}")),
        }
    }
    report.finish();
}
