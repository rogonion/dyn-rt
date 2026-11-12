//! Data-driven suite for the json-schema ADAPTERS layer (ADR 0004/0009):
//! src/adapters/mod.rs — schema-path resolution from JSONPath strings.

#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "deser")]
mod deser;

use common::*;
use dyn_rt_json_schema::Schema;
use dyn_rt_json_schema::adapters::schema_at_path_str;
use dyn_rt_reflect::ReflectKind;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

struct AdapterCase {
    meta: TestMetadata,
    path: &'static str,
    schema: Schema,
    /// Ok((kind, type_name)) or Err(message fragment)
    expected: Result<(ReflectKind, String), &'static str>,
}

macro_rules! adapter_case {
    ($path:expr, $schema:expr, $expected:expr $(, test_title = $title:expr)?) => {
        AdapterCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            path: $path,
            schema: $schema,
            expected: $expected,
        }
    };
}

#[test]
fn schema_at_path_str_cases() {
    let test_data: Vec<_> = vec![
        adapter_case!(
            "$.Name",
            Schema::Node(user_schema()),
            Ok((
                ReflectKind::Scalar,
                std::any::type_name::<String>().to_string()
            )),
            test_title = "str path resolves a struct field via the tokenizer adapter"
        ),
        adapter_case!(
            "$.Employees[0].Name",
            Schema::Node(company_schema()),
            Ok((
                ReflectKind::Scalar,
                std::any::type_name::<String>().to_string()
            )),
            test_title = "str path with index through indirection"
        ),
        adapter_case!(
            "$..Name",
            Schema::Node(user_schema()),
            Err("multiple recursive descent"),
            test_title = "recursive descent paths are rejected by the str adapter"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let got = schema_at_path_str(case.path, &case.schema);
        match (&case.expected, &got) {
            (Ok((kind, type_name)), Ok(resolved)) => {
                let got_kind = resolved.node.kind();
                let got_type = resolved.node.type_info.type_name.to_string();
                if &got_kind != kind || &got_type != type_name {
                    failures.push(format!(
                        "{title} || expected ({:?}, {:?}), got ({:?}, {:?})",
                        kind, type_name, got_kind, got_type
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
    report_failures(failures, test_data.len());
}
