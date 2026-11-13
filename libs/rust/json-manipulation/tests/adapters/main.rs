//! Data-driven suite for the json-manipulation ADAPTERS layer (ADR 0004/0009):
//! src/adapters/mod.rs — the string-path entry points (parse via json-path's
//! regex tokenizer, then delegate to the application use cases).

#[path = "../common/mod.rs"]
mod common;

use common::*;
use dyn_rt_json_manipulation::adapters::{delete_str, get_str, set_str};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

struct AdapterCase {
    test_title: Option<String>,
    root: Box<dyn Reflect>,
    path: &'static str,
    /// "get" expects rendered results; "set"/"delete" expect count + rendered root
    op: &'static str,
    value: Option<Box<dyn Reflect>>,
    expected: Vec<String>,
    expected_root: String,
    expected_count: usize,
}

macro_rules! adapter_case {
    ($op:expr, $root:expr, $path:expr, [$($expected:expr),*] $(, test_title = $title:expr)?) => {
        AdapterCase {
            test_title: None $(.or(Some($title.to_string())))?,
            root: Box::new($root),
            path: $path,
            op: $op,
            value: None,
            expected: vec![$($expected.to_string()),*],
            expected_root: String::new(),
            expected_count: 0,
        }
    };
    ($op:expr, $root:expr, $path:expr, $value:expr, $count:expr, $expected_root:expr $(, test_title = $title:expr)?) => {
        AdapterCase {
            test_title: None $(.or(Some($title.to_string())))?,
            root: Box::new($root),
            path: $path,
            op: $op,
            value: Some(Box::new($value)),
            expected: Vec::new(),
            expected_root: $expected_root,
            expected_count: $count,
        }
    };
}

#[test]
fn string_path_adapter_cases() {
    let test_data: Vec<AdapterCase> = vec![
        adapter_case!(
            "get",
            Value::object([("a", Value::object([("b", Value::from(1i64))]))]),
            "$.a.b",
            [format!("{:?}", Value::from(1i64))],
            test_title = "get_str parses and gets a nested key path"
        ),
        adapter_case!(
            "get",
            Value::from(vec![Value::from(1i64), Value::from(2i64)]),
            "$[*]",
            [
                format!("{:?}", Value::from(1i64)),
                format!("{:?}", Value::from(2i64))
            ],
            test_title = "get_str with wildcard over a sequence"
        ),
        adapter_case!(
            "set",
            Value::object([("a", Value::from(1i64))]),
            "$.a",
            Value::from(2i64),
            1,
            format!("{:?}", Value::object([("a", Value::from(2i64))])),
            test_title = "set_str parses and sets a key path"
        ),
        adapter_case!(
            "delete",
            Value::object([("a", Value::from(1i64)), ("b", Value::from(2i64))]),
            "$.a",
            Value::Null,
            1,
            format!("{:?}", Value::object([("b", Value::from(2i64))])),
            test_title = "delete_str parses and deletes a key path"
        ),
    ];

    let mut report = TestReport::new(
        "string_path_adapter_cases",
        "string-path entry points over the application use cases (src/adapters/mod.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let mut root = case.root.reflect_clone();
        match case.op {
            "get" => match get_str(&*root, case.path) {
                Ok(values) => {
                    let rendered: Vec<String> = values.iter().map(|v| render(*v)).collect();
                    if rendered != case.expected {
                        report
                            .fail_case(format!("expected {:?}, got {:?}", case.expected, rendered));
                    }
                }
                Err(err) => report.fail_case(format!("unexpected error: {err}")),
            },
            "set" => {
                let value = case.value.as_ref().expect("set case carries a value");
                match set_str(&mut *root, case.path, &**value, None) {
                    Ok(count) => {
                        if count != case.expected_count {
                            report.fail_case(format!(
                                "expected count {}, got {}",
                                case.expected_count, count
                            ));
                        }
                        let rendered = render(&*root);
                        if rendered != case.expected_root {
                            report.fail_case(format!(
                                "expected root {}, got {}",
                                case.expected_root, rendered
                            ));
                        }
                    }
                    Err(err) => report.fail_case(format!("unexpected error: {err}")),
                }
            }
            "delete" => match delete_str(&mut *root, case.path) {
                Ok(count) => {
                    if count != case.expected_count {
                        report.fail_case(format!(
                            "expected count {}, got {}",
                            case.expected_count, count
                        ));
                    }
                    let rendered = render(&*root);
                    if rendered != case.expected_root {
                        report.fail_case(format!(
                            "expected root {}, got {}",
                            case.expected_root, rendered
                        ));
                    }
                }
                Err(err) => report.fail_case(format!("unexpected error: {err}")),
            },
            other => report.fail_case(format!("unknown op {other:?}")),
        }
    }
    report.finish();
}

#[test]
fn for_each_str_cases() {
    use dyn_rt_json_manipulation::adapters::for_each_str;
    use std::cell::RefCell;

    struct ForEachAdapterCase {
        test_title: Option<String>,
        root: Box<dyn Reflect>,
        path: &'static str,
        expected_visited: Vec<String>,
    }

    macro_rules! for_each_adapter_case {
        ($root:expr, $path:expr, [$($expected:expr),*] $(, test_title = $title:expr)?) => {
            ForEachAdapterCase {
                test_title: None $(.or(Some($title.to_string())))?,
                root: Box::new($root),
                path: $path,
                expected_visited: vec![$($expected.to_string()),*],
            }
        };
    }

    let test_data: Vec<ForEachAdapterCase> = vec![
        for_each_adapter_case!(
            Value::object([("a", Value::from(1i64)), ("b", Value::from(2i64)),]),
            "$.*",
            [
                format!("{:?}", Value::from(1i64)),
                format!("{:?}", Value::from(2i64))
            ],
            test_title = "for_each_str parses and visits wildcard matches"
        ),
        for_each_adapter_case!(
            Value::from(vec![Value::from(5i64), Value::from(6i64)]),
            "$[0,-1]",
            [
                format!("{:?}", Value::from(5i64)),
                format!("{:?}", Value::from(6i64))
            ],
            test_title = "for_each_str over union with negative index member"
        ),
    ];

    let mut report = TestReport::new(
        "for_each_str_cases",
        "string-path for_each entry point (src/adapters/mod.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let visited: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let result = for_each_str(&*case.root, case.path, &mut |value| {
            visited.borrow_mut().push(render(value));
            true
        });
        match result {
            Ok(()) => {
                let visited = visited.borrow();
                if *visited != case.expected_visited {
                    report.fail_case(format!(
                        "path {} || expected {:?}, got {:?}",
                        case.path, case.expected_visited, *visited
                    ));
                }
            }
            Err(err) => report.fail_case(format!("unexpected error: {err}")),
        }
    }
    report.finish();
}
