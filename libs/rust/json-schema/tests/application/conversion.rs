//! Data-driven suite for src/application/conversion.rs (ADR 0004/0009):
//! converting reflected data into a target as dictated by a schema.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use crate::common::*;
use dyn_rt_json_schema::{
    ConversionOptions, Schema, SchemaConverter, SchemaError, SchemaNode, SchemaPathSegment,
    convert_with,
};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

/// Renders a reflected value by downcasting to the concrete test type.
type RenderFn = Box<dyn Fn(&dyn Reflect) -> String>;

fn debug_downcast<T: Reflect + Debug + 'static>(r: &dyn Reflect) -> String {
    match r.reflect_as_any().downcast_ref::<T>() {
        Some(v) => format!("{v:?}"),
        None => format!("<not {}>", std::any::type_name::<T>()),
    }
}

/// A custom converter doubling any i64 source (mirrors go's custom Converters).
struct DoubleI64;

impl SchemaConverter for DoubleI64 {
    fn convert(
        &self,
        data: &dyn Reflect,
        _node: &SchemaNode,
        _path: &[SchemaPathSegment],
    ) -> Result<Box<dyn Reflect>, SchemaError> {
        let value = data
            .reflect_as_any()
            .downcast_ref::<i64>()
            .expect("registered for i64");
        Ok(Box::new(value * 2))
    }
}

struct ConversionCase {
    meta: TestMetadata,
    source: Box<dyn Reflect>,
    schema: Schema,
    make_target: fn() -> Box<dyn Reflect>,
    render: RenderFn,
    options: ConversionOptions,
    /// Ok(expected render of target after conversion, plus valid set keys)
    /// or Err(message fragment)
    expected: Result<(String, Vec<String>), &'static str>,
}

macro_rules! conversion_case {
    ($source:expr, $schema:expr, $target:expr, $render:expr, $expected:expr $(, test_title = $title:expr)?) => {
        ConversionCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            source: Box::new($source),
            schema: $schema,
            make_target: $target,
            render: Box::new($render),
            options: ConversionOptions::new(),
            expected: $expected,
        }
    };
}

fn user_value() -> Value {
    Value::object([
        ("ID", Value::from(1i64)),
        ("Name", Value::from("amy")),
        ("Email", Value::from("amy@example.com")),
    ])
}

fn empty_user() -> Box<dyn Reflect> {
    Box::new(User::default())
}

#[test]
fn conversion_cases() {
    let test_data: Vec<_> = vec![
        conversion_case!(
            "42".to_string(),
            Schema::Node(SchemaNode::scalar::<i64>()),
            || Box::new(0i64),
            debug_downcast::<i64>,
            Ok(("42".to_string(), vec![])),
            test_title = "string coerces to i64"
        ),
        conversion_case!(
            5i64,
            Schema::Node(SchemaNode::scalar::<String>()),
            || Box::new(String::new()),
            debug_downcast::<String>,
            Ok(("\"5\"".to_string(), vec![])),
            test_title = "i64 renders to String"
        ),
        conversion_case!(
            1i64,
            Schema::Node(SchemaNode::scalar::<bool>()),
            || Box::new(false),
            debug_downcast::<bool>,
            Ok(("true".to_string(), vec![])),
            test_title = "non-zero int coerces to bool true"
        ),
        conversion_case!(
            "2.5".to_string(),
            Schema::Node(SchemaNode::scalar::<f64>()),
            || Box::new(0.0f64),
            debug_downcast::<f64>,
            Ok(("2.5".to_string(), vec![])),
            test_title = "string parses to f64"
        ),
        conversion_case!(
            Value::from(7i64),
            Schema::Node(SchemaNode::scalar::<i64>()),
            || Box::new(0i64),
            debug_downcast::<i64>,
            Ok(("7".to_string(), vec![])),
            test_title = "json number converts to i64"
        ),
        conversion_case!(
            Value::from("x"),
            Schema::Node(SchemaNode::json_scalar("string")),
            || Box::new(Value::Null),
            debug_downcast::<Value>,
            Ok(("String(\"x\")".to_string(), vec![])),
            test_title = "json string variant round-trips through a json string node"
        ),
        conversion_case!(
            user_value(),
            Schema::Node(user_schema()),
            empty_user,
            debug_downcast::<User>,
            Ok((format!("{:?}", sample_user()), vec![])),
            test_title = "Value object converts into a struct target field by field"
        ),
        conversion_case!(
            sample_user(),
            Schema::Node(user_schema()),
            empty_user,
            debug_downcast::<User>,
            Ok((format!("{:?}", sample_user()), vec![])),
            test_title = "struct source converts into a struct target"
        ),
        conversion_case!(
            Value::object([
                ("Name", Value::from("amy")),
                ("Age", Value::from(30i64)),
                (
                    "Address",
                    Value::object([
                        ("Street", Value::from("1 Main St")),
                        ("City", Value::from("Springfield")),
                        ("ZipCode", Value::from("12345")),
                    ])
                ),
            ]),
            Schema::Node(user_profile_schema()),
            || Box::new(UserProfile::default()),
            debug_downcast::<UserProfile>,
            Ok((format!("{:?}", sample_user_profile()), vec![])),
            test_title = "nested Value object converts into nested struct (incl. Option field)"
        ),
        conversion_case!(
            Value::Array(vec![Value::object([
                ("ID", Value::from(7i64)),
                ("Name", Value::from("widget")),
                ("Price", Value::from(9.99f64)),
            ])]),
            Schema::Node(list_of_products_schema()),
            || Box::new(Vec::<Product>::new()),
            debug_downcast::<Vec<Product>>,
            Ok((format!("{:?}", vec![sample_product()]), vec![])),
            test_title = "Value array converts into a sequence of structs"
        ),
        conversion_case!(
            Value::object([("1", user_value())]),
            Schema::Node(map_user_schema()),
            || Box::new(HashMap::<i64, Box<User>>::new()),
            debug_downcast::<HashMap<i64, Box<User>>>,
            Ok((
                format!("{:?}", HashMap::from([(1i64, Box::new(sample_user()))])),
                vec![]
            )),
            test_title = "Value object converts into map[int]Box<User> (string key coerced to int, value through indirection)"
        ),
        conversion_case!(
            Value::object([
                ("Name", Value::from("amy")),
                (
                    "Address",
                    Value::object([
                        ("Street", Value::from("1 Main St")),
                        ("City", Value::from("Springfield")),
                        ("ZipCode", Value::Null),
                    ])
                ),
            ]),
            Schema::Node(user_with_address_schema()),
            || Box::new(UserWithAddress {
                name: String::new(),
                address: Box::new(Address::default())
            }),
            debug_downcast::<UserWithAddress>,
            Ok((
                format!(
                    "{:?}",
                    UserWithAddress {
                        name: "amy".to_string(),
                        address: Box::new(Address {
                            street: "1 Main St".to_string(),
                            city: "Springfield".to_string(),
                            zip_code: None
                        })
                    }
                ),
                vec![]
            )),
            test_title = "indirection field converts through Box (null zip stays None via nilable)"
        ),
        conversion_case!(
            user_value(),
            Schema::Set(dyn_rt_json_schema::SchemaSet::new(
                Some("User"),
                vec![("User", user_schema()), ("Product", product_schema()),],
            )),
            empty_user,
            debug_downcast::<User>,
            Ok((format!("{:?}", sample_user()), vec!["User".to_string()])),
            test_title = "polymorphic set converts via the matching node and reports the key"
        ),
        conversion_case!(
            42i64,
            Schema::Node(user_schema()),
            empty_user,
            debug_downcast::<User>,
            Err("unsupported source kind"),
            test_title = "scalar source cannot convert into a struct target"
        ),
        conversion_case!(
            Option::<String>::None,
            Schema::Node(
                SchemaNode::enum_node::<Option<String>>(vec![
                    ("Some", Some(Schema::Node(string_node()))),
                    ("None", None),
                ])
                .nilable()
            ),
            || Box::new(Some("keep".to_string())),
            debug_downcast::<Option<String>>,
            Ok(("Some(\"keep\")".to_string(), vec![])),
            test_title = "nil source against a nilable node leaves the target untouched"
        ),
        conversion_case!(
            Option::<String>::None,
            Schema::Node(SchemaNode::enum_node::<Option<String>>(vec![
                ("Some", Some(Schema::Node(string_node()))),
                ("None", None),
            ])),
            || Box::new(Some("keep".to_string())),
            debug_downcast::<Option<String>>,
            Err("source cannot be nil"),
            test_title = "nil source against a non-nilable node errors"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut target = (case.make_target)();
        let got = convert_with(&*case.source, &case.schema, &mut *target, &case.options);
        match (&case.expected, &got) {
            (Ok((expected_render, expected_keys)), Ok(outcome)) => {
                let got_render = (case.render)(&*target);
                if &got_render != expected_render || &outcome.valid_set_keys != expected_keys {
                    failures.push(format!(
                        "{title} || expected ({:?}, {:?}), got ({:?}, {:?})",
                        expected_render, expected_keys, got_render, outcome.valid_set_keys
                    ));
                }
            }
            (Err(fragment), Err(err)) => {
                if !err.to_string().contains(fragment) {
                    failures.push(format!(
                        "{title} || error: expected fragment {:?}, got {:?}",
                        fragment, err
                    ));
                }
            }
            _ => failures.push(format!(
                "{title} || expected {:?}, got {:?}",
                case.expected, got
            )),
        }
    }

    // custom converter registry case (go's WithCustomConverters, keyed by source type)
    let options =
        ConversionOptions::new().with_custom_converter(TypeId::of::<i64>(), Arc::new(DoubleI64));
    let mut target: Box<dyn Reflect> = Box::new(0i64);
    let got = convert_with(
        &5i64,
        &Schema::Node(SchemaNode::scalar::<i64>()),
        &mut *target,
        &options,
    );
    match got {
        Ok(_) if debug_downcast::<i64>(&*target) == "10" => {}
        other => failures.push(format!(
            "Test case custom: custom converter doubles i64 || expected \"10\", got {:?}",
            other.map(|o| (debug_downcast::<i64>(&*target), o.valid_set_keys))
        )),
    }

    report_failures(failures, test_data.len() + 1);
}
