//! Data-driven suite for src/application/object.rs (ADR 0004/0009): the
//! `DynObject` entrypoint wrapper — text vs parsed-segment path inputs,
//! borrowed vs owned source, schema attachment, get/set/delete/for_each.

use dyn_rt_manipulation::application::{DynObject, are_equal};
use dyn_rt_path::{RecursiveDescentSegments, RegexPathTokenizer, parse_path};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

fn value_source() -> Box<dyn Reflect> {
    Box::new(Value::object([
        (
            "a",
            Value::object([
                ("b", Value::from(1i64)),
                ("c", Value::from(vec![Value::from(2i64), Value::from(3i64)])),
            ]),
        ),
        ("name", Value::from("Alice")),
    ]))
}

fn source_with_name(name: &str) -> Box<dyn Reflect> {
    Box::new(Value::object([
        (
            "a",
            Value::object([
                ("b", Value::from(1i64)),
                ("c", Value::from(vec![Value::from(2i64), Value::from(3i64)])),
            ]),
        ),
        ("name", Value::from(name)),
    ]))
}

fn source_without_a_b() -> Box<dyn Reflect> {
    Box::new(Value::object([
        (
            "a",
            Value::object([("c", Value::from(vec![Value::from(2i64), Value::from(3i64)]))]),
        ),
        ("name", Value::from("Alice")),
    ]))
}

/// One DynObject interaction, applied to a fresh copy of `make_source`.
struct ObjectCase {
    test_title: Option<String>,
    make_source: fn() -> Box<dyn Reflect>,
    /// path in text form (also parsed for the parsed-segment variant)
    path: &'static str,
    op: ObjectOp,
    /// expected get results (compared with are_equal)
    expected_values: Vec<Box<dyn Reflect>>,
    /// expected source after the op (compared with are_equal)
    expected_source: Option<Box<dyn Reflect>>,
    /// expected for_each visited scalar strings (joined with ",")
    expected_visited: Option<String>,
    /// when set, the op must fail with a message containing this fragment
    expect_error: Option<&'static str>,
}

enum ObjectOp {
    Get,
    Set(Box<dyn Reflect>),
    Delete,
    ForEach,
}

fn get_case(
    test_title: &str,
    path: &'static str,
    expected_values: Vec<Box<dyn Reflect>>,
) -> ObjectCase {
    ObjectCase {
        test_title: Some(test_title.to_string()),
        make_source: value_source,
        path,
        op: ObjectOp::Get,
        expected_values,
        expected_source: None,
        expected_visited: None,
        expect_error: None,
    }
}

#[test]
fn dyn_object_cases() {
    let test_data: Vec<ObjectCase> = vec![
        get_case(
            "get with a text path",
            "$.a.b",
            vec![Box::new(Value::from(1i64))],
        ),
        get_case(
            "get with a wildcard text path",
            "$.a.c[*]",
            vec![Box::new(Value::from(2i64)), Box::new(Value::from(3i64))],
        ),
        ObjectCase {
            test_title: Some("set with a text path updates the source".to_string()),
            make_source: value_source,
            path: "$.name",
            op: ObjectOp::Set(Box::new(Value::from("Bob"))),
            expected_values: Vec::new(),
            expected_source: Some(source_with_name("Bob")),
            expected_visited: None,
            expect_error: None,
        },
        ObjectCase {
            test_title: Some("delete with a text path removes the member".to_string()),
            make_source: value_source,
            path: "$.a.b",
            op: ObjectOp::Delete,
            expected_values: Vec::new(),
            expected_source: Some(source_without_a_b()),
            expected_visited: None,
            expect_error: None,
        },
        ObjectCase {
            test_title: Some("for_each visits every wildcard match".to_string()),
            make_source: value_source,
            path: "$.a.c[*]",
            op: ObjectOp::ForEach,
            expected_values: Vec::new(),
            expected_source: None,
            expected_visited: Some("2,3".to_string()),
            expect_error: None,
        },
    ];

    let mut report = TestReport::new(
        "dyn_object_cases",
        "DynObject wrapper: text path reps + get/set/delete/for_each (src/application/object.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let mut source = (case.make_source)();
        let mut obj = DynObject::new(&mut *source);
        let result: Result<(), String> = (|| match &case.op {
            ObjectOp::Get => {
                let found = obj.get(case.path).map_err(|e| e.to_string())?;
                if found.len() != case.expected_values.len() {
                    return Err(format!(
                        "expected {} value(s), got {}",
                        case.expected_values.len(),
                        found.len()
                    ));
                }
                for (f, e) in found.iter().zip(case.expected_values.iter()) {
                    if !are_equal(*f, &**e) {
                        return Err("value mismatch".to_string());
                    }
                }
                Ok(())
            }
            ObjectOp::Set(value) => {
                obj.set(case.path, &**value).map_err(|e| e.to_string())?;
                Ok(())
            }
            ObjectOp::Delete => {
                obj.delete(case.path).map_err(|e| e.to_string())?;
                Ok(())
            }
            ObjectOp::ForEach => {
                let mut visited: Vec<String> = Vec::new();
                obj.for_each(case.path, &mut |v| {
                    if let Some(s) = v.reflect_scalar_string() {
                        visited.push(s);
                    } else if let Some(n) = v.reflect_scalar_f64() {
                        visited.push(n.to_string());
                    }
                    true
                })
                .map_err(|e| e.to_string())?;
                if Some(visited.join(",")) != case.expected_visited {
                    return Err(format!("visited {:?}", visited));
                }
                Ok(())
            }
        })();
        match (&result, case.expect_error) {
            (Err(detail), Some(needle)) => {
                if !detail.contains(needle) {
                    report.fail_case(format!("error {detail:?} missing {needle:?}"));
                }
            }
            (Err(detail), None) => report.fail_case(detail.clone()),
            (Ok(()), Some(_)) => report.fail_case("expected an error, got Ok"),
            (Ok(()), None) => {
                if let Some(expected) = &case.expected_source {
                    if !are_equal(&*source, &**expected) {
                        report.fail_case("source mismatch after op".to_string());
                    }
                }
            }
        }
    }
    report.finish();
}

/// Parsed-segment path input (instead of text) resolves the same way.
#[test]
fn dyn_object_parsed_segment_case() {
    let mut report = TestReport::new(
        "dyn_object_parsed_segment_case",
        "DynObject accepts RecursiveDescentSegments directly (src/application/object.rs)",
    );
    report.begin_case("get via parsed segments matches get via text");
    let mut source = value_source();
    let obj = DynObject::new(&mut *source);
    let segments: RecursiveDescentSegments = parse_path(&RegexPathTokenizer, "$.a.b");
    let via_segments = obj.get(&segments);
    let via_text = obj.get("$.a.b");
    match (via_segments, via_text) {
        (Ok(a), Ok(b)) => {
            if a.len() != 1
                || b.len() != 1
                || !are_equal(a[0], b[0])
                || !are_equal(a[0], &Value::from(1i64))
            {
                report.fail_case("parsed-segment and text gets disagree".to_string());
            }
        }
        _ => report.fail_case("get failed".to_string()),
    }
    report.finish();
}

/// An owned source needs no external owner and comes back out with into_source.
#[test]
fn dyn_object_owned_source_case() {
    let mut report = TestReport::new(
        "dyn_object_owned_source_case",
        "DynObject::owned wraps an owned source (src/application/object.rs)",
    );
    report.begin_case("owned source: set then into_source returns the modified value");
    let mut obj = DynObject::owned(value_source());
    match obj.set("$.name", &Value::from("Carol")) {
        Ok(count) => {
            if count != 1 {
                report.fail_case(format!("expected 1 set, got {count}"));
            }
        }
        Err(err) => {
            report.fail_case(format!("set failed: {err}"));
            report.finish();
            return;
        }
    }
    let source = obj.into_source();
    if !are_equal(&*source, &*source_with_name("Carol")) {
        report.fail_case("owned source was not modified as expected".to_string());
    }
    report.finish();
}
