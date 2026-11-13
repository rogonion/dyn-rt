//! Application suite for `src/application/are_equal.rs`: recursive structural
//! equality over reflected values, incl. custom per-TypeId checkers.

use std::any::TypeId;
use std::collections::HashMap;

use crate::common::*;
use dyn_rt_json_manipulation::{CustomEqualFn, are_equal, are_equal_with};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

/// Custom checker for the custom-checker case: two `UserProfile`s are "equal"
/// when only the name matches (ids ignored).
fn name_only(left: &dyn Reflect, right: &dyn Reflect) -> bool {
    match (
        left.reflect_as_any().downcast_ref::<UserProfile>(),
        right.reflect_as_any().downcast_ref::<UserProfile>(),
    ) {
        (Some(l), Some(r)) => l.name == r.name,
        _ => false,
    }
}

struct AreEqualCase {
    test_title: Option<String>,
    left: Box<dyn Reflect>,
    right: Box<dyn Reflect>,
    expected: bool,
    custom: Option<HashMap<TypeId, CustomEqualFn>>,
}

macro_rules! are_equal_case {
    ($left:expr, $right:expr, $expected:expr $(, custom = $custom:expr)? $(, test_title = $title:expr)?) => {
        AreEqualCase {
            test_title: None $(.or(Some($title.to_string())))?,
            left: Box::new($left),
            right: Box::new($right),
            expected: $expected,
            custom: None $(.or(Some($custom)))?,
        }
    };
}

#[test]
fn are_equal_cases() {
    let mut custom_map: HashMap<TypeId, CustomEqualFn> = HashMap::new();
    custom_map.insert(TypeId::of::<UserProfile>(), name_only);

    let test_data: Vec<AreEqualCase> = vec![
        are_equal_case!(
            Value::from(1i64),
            Value::from(1i64),
            true,
            test_title = "equal scalars"
        ),
        are_equal_case!(
            Value::from(1i64),
            Value::from(2i64),
            false,
            test_title = "unequal scalars"
        ),
        are_equal_case!(
            Value::from(1i64),
            Value::from(1f64),
            false,
            test_title = "int vs float of same numeric value are different concrete types"
        ),
        are_equal_case!(
            Value::from(vec![Value::from(1i64), Value::from("x")]),
            Value::from(vec![Value::from(1i64), Value::from("x")]),
            true,
            test_title = "equal sequences"
        ),
        are_equal_case!(
            Value::from(vec![Value::from(1i64)]),
            Value::from(vec![Value::from(1i64), Value::from(2i64)]),
            false,
            test_title = "sequences of different length"
        ),
        // map equality is key-matched, so insertion order does not matter
        are_equal_case!(
            Value::object([("a", Value::from(1i64)), ("b", Value::from(2i64))]),
            Value::object([("b", Value::from(2i64)), ("a", Value::from(1i64))]),
            true,
            test_title = "maps equal regardless of entry order"
        ),
        are_equal_case!(
            Value::object([("a", Value::from(1i64))]),
            Value::object([("a", Value::from(2i64))]),
            false,
            test_title = "maps with different values"
        ),
        are_equal_case!(
            Value::object([("a", Value::from(1i64))]),
            Value::object([("b", Value::from(1i64))]),
            false,
            test_title = "maps with different keys"
        ),
        are_equal_case!(
            UserProfile {
                name: "Ann".to_string(),
                age: 30,
                ..Default::default()
            },
            UserProfile {
                name: "Ann".to_string(),
                age: 30,
                ..Default::default()
            },
            true,
            test_title = "equal structs"
        ),
        are_equal_case!(
            UserProfile {
                name: "Ann".to_string(),
                age: 30,
                ..Default::default()
            },
            UserProfile {
                name: "Ann".to_string(),
                age: 31,
                ..Default::default()
            },
            false,
            test_title = "structs differing in one field"
        ),
        are_equal_case!(
            UserProfile {
                name: "Ann".to_string(),
                age: 30,
                ..Default::default()
            },
            UserProfile {
                name: "Ann".to_string(),
                age: 99,
                ..Default::default()
            },
            true,
            custom = custom_map.clone(),
            test_title = "custom per-TypeId checker overrides the default comparison"
        ),
        are_equal_case!(
            Some(Value::from(1i64)),
            Some(Value::from(1i64)),
            true,
            test_title = "Some payloads equal"
        ),
        are_equal_case!(
            Some(Value::from(1i64)),
            None::<Value>,
            false,
            test_title = "Some vs None"
        ),
        are_equal_case!(
            None::<Value>,
            None::<Value>,
            true,
            test_title = "None vs None"
        ),
        are_equal_case!(
            Box::new(Value::from(7i64)),
            Box::new(Value::from(7i64)),
            true,
            test_title = "boxed values unwrap and compare"
        ),
        are_equal_case!(
            (1i64, "a".to_string()),
            (1i64, "a".to_string()),
            true,
            test_title = "equal tuples (struct shape, positional fields)"
        ),
        are_equal_case!(
            (1i64, "a".to_string()),
            (1i64, "b".to_string()),
            false,
            test_title = "tuples differing in one element"
        ),
        are_equal_case!((), (), true, test_title = "unit equals unit"),
        are_equal_case!(
            (),
            Value::Null,
            false,
            test_title = "unit vs Value::Null are different concrete types"
        ),
        are_equal_case!(
            ShapeEnum::Circle(Circle { radius: 2.5 }),
            ShapeEnum::Circle(Circle { radius: 2.5 }),
            true,
            test_title = "enum variants with equal payloads"
        ),
        are_equal_case!(
            ShapeEnum::Circle(Circle { radius: 2.5 }),
            ShapeEnum::Square(Square { side: 2.5 }),
            false,
            test_title = "different enum variants are unequal even with equal-ish payloads"
        ),
        are_equal_case!(
            std::rc::Rc::new(User {
                id: 1,
                name: "Ann".to_string(),
                email: "a@x".to_string()
            }),
            User {
                id: 1,
                name: "Ann".to_string(),
                email: "a@x".to_string()
            },
            true,
            test_title = "Rc<T> unwraps and compares against the bare T"
        ),
        are_equal_case!(
            [1i64, 2, 3],
            [1i64, 2, 3],
            true,
            test_title = "fixed-size arrays compare element-wise"
        ),
    ];

    let mut report = TestReport::new(
        "are_equal_cases",
        "recursive structural equality over reflected values (src/application/are_equal.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let got = match &case.custom {
            Some(custom) => are_equal_with(&*case.left, &*case.right, Some(custom)),
            None => are_equal(&*case.left, &*case.right),
        };
        if got != case.expected {
            report.fail_case(format!(
                "expected {got_expected}, got {got}",
                got_expected = case.expected
            ));
        }
    }
    report.finish();
}
