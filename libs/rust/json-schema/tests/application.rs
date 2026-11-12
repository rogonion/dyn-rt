//! Data-driven suite for the json-schema APPLICATION layer (ADR 0004/0009):
//! validation. Suite covers src/application.rs (validate / validate_with /
//! validate_node). Fixtures mirror go-json's schema/misc_test.go shapes.

#[path = "common/mod.rs"]
mod common;

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use common::*;
use dyn_rt_json_schema::{
    Schema, SchemaError, SchemaNode, SchemaValidator, ValidationOptions, validate_with,
};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

/// A custom validator accepting only even i64 values (mirrors go's custom
/// Validators registry keyed by type).
struct EvenValidator;

impl SchemaValidator for EvenValidator {
    fn validate(
        &self,
        data: &dyn Reflect,
        _node: &SchemaNode,
        _path: &[dyn_rt_json_schema::SchemaPathSegment],
    ) -> Result<(), SchemaError> {
        let value = data
            .reflect_as_any()
            .downcast_ref::<i64>()
            .expect("registered for i64");
        if value % 2 == 0 {
            Ok(())
        } else {
            Err(SchemaError::ValidationFailed {
                message: format!("{value} is not even"),
                path: "$".to_string(),
            })
        }
    }
}

struct ValidationCase {
    meta: TestMetadata,
    data: Box<dyn Reflect>,
    schema: Schema,
    options: ValidationOptions,
    /// Ok(expected valid set keys) or Err(message fragment expected in the error)
    expected: Result<Vec<String>, &'static str>,
}

macro_rules! validation_case {
    ($data:expr, $schema:expr, $expected:expr $(, test_title = $title:expr)?) => {
        ValidationCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            data: Box::new($data),
            schema: $schema,
            options: ValidationOptions::new(),
            expected: $expected,
        }
    };
}

#[test]
fn validation_cases() {
    let mut strict_user_node = user_schema();
    strict_user_node.children_must_be_valid = true;
    if let dyn_rt_json_schema::SchemaShape::Struct { fields } = &mut strict_user_node.shape {
        fields.push(("Nickname".to_string(), Schema::Node(string_node())));
    }
    let strict_user_schema = Schema::Node(strict_user_node);

    let test_data: Vec<_> = vec![
        validation_case!(
            "this is a string".to_string(),
            Schema::Node(SchemaNode::scalar::<String>()),
            Ok(vec![]),
            test_title = "validate simple primitive"
        ),
        validation_case!(
            42i64,
            Schema::Node(SchemaNode::scalar::<String>()),
            Err("not valid"),
            test_title = "validate invalid simple primitive"
        ),
        validation_case!(
            sample_user(),
            Schema::Node(user_schema()),
            Ok(vec![]),
            test_title = "validate struct against its schema"
        ),
        validation_case!(
            sample_user(),
            strict_user_schema.clone(),
            Err("not all child nodes"),
            test_title = "children_must_be_valid fails when a schema field is absent from data"
        ),
        validation_case!(
            Value::object([("a", Value::from(1i64)), ("b", Value::from("x"))]),
            Schema::Node(json_map_schema()),
            Ok(vec![]),
            test_title = "validate Value object against map[string]any schema"
        ),
        validation_case!(
            HashMap::from([(1i64, Value::from("x"))]),
            Schema::Node(json_map_schema()),
            Err("not valid"),
            test_title = "int map key is invalid against a json string key schema"
        ),
        validation_case!(
            sample_nested_item(),
            Schema::Node(nested_item_schema()),
            Ok(vec![]),
            test_title = "validate nested item (map + list of any)"
        ),
        validation_case!(
            vec![sample_product(), sample_product()],
            Schema::Node(list_of_products_schema()),
            Ok(vec![]),
            test_title = "validate list of products"
        ),
        validation_case!(
            vec![Value::from("ok"), Value::from(5i64)],
            Schema::Node(SchemaNode::sequence_node::<Vec<Value>>(Schema::Node(
                SchemaNode::json_scalar("string")
            ))),
            Err("not valid"),
            test_title = "list with a non-string element fails at the element schema"
        ),
        validation_case!(
            sample_employee(),
            Schema::Node(employee_schema()),
            Ok(vec![]),
            test_title = "validate employee (nested struct, string list, string->int map)"
        ),
        validation_case!(
            sample_company(),
            Schema::Node(company_schema()),
            Ok(vec![]),
            test_title = "validate company (list of boxed users)"
        ),
        validation_case!(
            sample_user_profile(),
            Schema::Node(user_profile_schema()),
            Ok(vec![]),
            test_title = "validate user profile with nested address"
        ),
        validation_case!(
            Address {
                street: "1 Main St".to_string(),
                city: "Springfield".to_string(),
                zip_code: None,
            },
            Schema::Node(address_schema()),
            Ok(vec![]),
            test_title = "nil zip code is valid against the nilable Option schema"
        ),
        validation_case!(
            Address {
                street: "1 Main St".to_string(),
                city: "Springfield".to_string(),
                zip_code: None,
            },
            Schema::Node({
                let mut node = address_schema();
                if let dyn_rt_json_schema::SchemaShape::Struct { fields } = &mut node.shape {
                    let (_, zip) = fields.iter_mut().find(|(n, _)| n == "ZipCode").unwrap();
                    if let Schema::Node(zip_node) = zip {
                        zip_node.nilable = false;
                    }
                }
                node
            }),
            Err("data cannot be nil"),
            test_title = "nil zip code fails against a non-nilable schema"
        ),
        validation_case!(
            Circle { radius: 1.5 },
            shape_schema(),
            Ok(vec!["Circle".to_string()]),
            test_title = "polymorphic set validates Circle and reports the matched key"
        ),
        validation_case!(
            Square { side: 2.0 },
            shape_schema(),
            Ok(vec!["Circle".to_string()]),
            test_title = "without children_must_be_valid, Square first-matches Circle (unknown fields are skipped, as in go)"
        ),
        validation_case!(
            Square { side: 2.0 },
            Schema::Set(dyn_rt_json_schema::SchemaSet::new(
                Some("default"),
                vec![
                    ("Circle", circle_schema().children_must_be_valid()),
                    ("Square", square_schema().children_must_be_valid()),
                ],
            )),
            Ok(vec!["Square".to_string()]),
            test_title = "with children_must_be_valid, Square matches only the Square node"
        ),
        validation_case!(
            Shape::Circle(Circle { radius: 1.5 }),
            Schema::Node(SchemaNode::enum_node::<Shape>(vec![
                ("Circle", Some(Schema::Node(circle_schema()))),
                ("Square", Some(Schema::Node(square_schema()))),
            ])),
            Ok(vec![]),
            test_title = "enum data validates against the enum node via its variant payload"
        ),
        validation_case!(
            42i64,
            shape_schema(),
            Err("not valid"),
            test_title = "scalar is not valid against the shape set"
        ),
        validation_case!(
            sample_user(),
            Schema::Node(SchemaNode::map_node::<HashMap<i64, Box<User>>>(
                Schema::Node(int_node()),
                Schema::Node(SchemaNode::indirection_node::<Box<User>>(Schema::Node(
                    user_schema()
                ))),
            )),
            Err("not valid"),
            test_title = "struct data is invalid against a map schema (kind mismatch)"
        ),
        validation_case!(
            HashMap::from([(1i64, Box::new(sample_user()))]),
            Schema::Node(map_user_schema()),
            Ok(vec![]),
            test_title = "map int -> boxed user validates against map_user schema"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let got = validate_with(&*case.data, &case.schema, &case.options);
        match (&case.expected, &got) {
            (Ok(expected_keys), Ok(outcome)) => {
                if &outcome.valid_set_keys != expected_keys {
                    failures.push(format!(
                        "{title} || valid set keys: expected {:?}, got {:?}",
                        expected_keys, outcome.valid_set_keys
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

    // custom validator registry case (go's WithCustomValidators)
    let custom = ValidationOptions::new()
        .with_custom_validator(TypeId::of::<i64>(), Arc::new(EvenValidator));
    let schema = Schema::Node(SchemaNode::scalar::<i64>());
    for (value, expected_ok, title) in [
        (4i64, true, "custom validator accepts even i64"),
        (3i64, false, "custom validator rejects odd i64"),
    ] {
        let got = validate_with(&value, &schema, &custom);
        if got.is_ok() != expected_ok {
            failures.push(format!(
                "Test case custom: {title} || expected ok={expected_ok}, got {:?}",
                got.map(|o| o.valid_set_keys)
            ));
        }
    }

    report_failures(failures, test_data.len() + 2);
}
