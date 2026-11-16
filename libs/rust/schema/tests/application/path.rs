//! Data-driven suite for src/application/path.rs: schema-path resolution
//! from parsed JSONPath segments (ADR 0004/0009).

use std::collections::HashMap;

use crate::common::*;
use dyn_rt_path::{RegexPathTokenizer, parse_path};
use dyn_rt_reflect::ReflectKind;
use dyn_rt_schema::{Schema, schema_at_path};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

struct PathCase {
    test_title: Option<String>,
    path: &'static str,
    schema: Schema,
    /// Ok((kind, type_name, valid_set_keys)) or Err(message fragment)
    expected: Result<(ReflectKind, String, Vec<String>), &'static str>,
}

macro_rules! path_case {
    ($path:expr, $schema:expr, $expected:expr $(, test_title = $title:expr)?) => {
        PathCase {
test_title: None $(.or(Some($title.to_string())))?,
            path: $path,
            schema: $schema,
            expected: $expected,
        }
    };
}

#[test]
fn schema_path_cases() {
    let test_data: Vec<_> = vec![
        path_case!(
            "$",
            Schema::Node(user_schema()),
            Ok((
                ReflectKind::Struct,
                std::any::type_name::<User>().to_string(),
                vec![]
            )),
            test_title = "root resolves to the schema root node"
        ),
        path_case!(
            "$.Name",
            Schema::Node(user_schema()),
            Ok((
                ReflectKind::Scalar,
                std::any::type_name::<String>().to_string(),
                vec![]
            )),
            test_title = "struct field resolves to its scalar node"
        ),
        path_case!(
            "$.Address.Street",
            Schema::Node(user_profile_schema()),
            Ok((
                ReflectKind::Scalar,
                std::any::type_name::<String>().to_string(),
                vec![]
            )),
            test_title = "nested struct field path"
        ),
        path_case!(
            "$.Employees[0].Name",
            Schema::Node(company_schema()),
            Ok((
                ReflectKind::Scalar,
                std::any::type_name::<String>().to_string(),
                vec![]
            )),
            test_title = "sequence index through an indirection into a struct field"
        ),
        path_case!(
            "$.MapData",
            Schema::Node(nested_item_schema()),
            Ok((
                ReflectKind::Map,
                std::any::type_name::<HashMap<String, Value>>().to_string(),
                vec![]
            )),
            test_title = "map-typed field resolves to the map node"
        ),
        path_case!(
            "$.ZipCode",
            Schema::Node(address_schema()),
            Ok((
                ReflectKind::Enum,
                std::any::type_name::<Option<String>>().to_string(),
                vec![]
            )),
            test_title = "Option field resolves to the enum node"
        ),
        path_case!(
            "$[0]",
            Schema::Node(list_of_products_schema()),
            Ok((
                ReflectKind::Struct,
                std::any::type_name::<Product>().to_string(),
                vec![]
            )),
            test_title = "sequence index resolves to the element schema"
        ),
        path_case!(
            "$[0]",
            Schema::Node(list_of_shapes_schema()),
            Ok((
                ReflectKind::Struct,
                std::any::type_name::<Circle>().to_string(),
                vec!["Circle".to_string()]
            )),
            test_title =
                "set-valued element resolves to its default/first node and records the key"
        ),
        path_case!(
            "$.Unknown",
            Schema::Node(user_schema()),
            Err("not found"),
            test_title = "unknown struct field errors"
        ),
    ];

    let mut report = TestReport::new(
        "schema_path_cases",
        "Data-driven suite for src/application/path.rs: schema-path resolution",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let parsed = parse_path(&RegexPathTokenizer, case.path);
        let got = schema_at_path(&parsed.0[0], &case.schema);
        match (&case.expected, &got) {
            (Ok((kind, type_name, keys)), Ok(resolved)) => {
                let got_kind = resolved.node.kind();
                let got_type = resolved.node.type_info.type_name.to_string();
                if &got_kind != kind || &got_type != type_name || &resolved.valid_set_keys != keys {
                    report.fail_case(format!(
                        "expected ({:?}, {:?}, {:?}), got ({:?}, {:?}, {:?})",
                        kind, type_name, keys, got_kind, got_type, resolved.valid_set_keys
                    ));
                }
            }
            (Err(fragment), Err(err)) => {
                if !err.to_string().contains(fragment) {
                    report.fail_case(format!(
                        "error: expected fragment {:?}, got {:?}",
                        fragment, err
                    ));
                }
            }
            _ => report.fail_case(format!("expected {:?}, got {:?}", case.expected, got)),
        }
    }
    report.finish();
}
