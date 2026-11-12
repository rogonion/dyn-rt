//! Data-driven suite for the json-schema DOMAIN layer (ADR 0004/0009):
//! schema model construction. Suite covers src/domain.rs.

#[path = "common/mod.rs"]
mod common;

use common::*;
use dyn_rt_json_schema::{Schema, SchemaNode};
use dyn_rt_reflect::ReflectKind;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

/// Projects a schema into a compact string for comparison.
fn project(schema: &Schema) -> String {
    match schema {
        Schema::Node(node) => project_node(node),
        Schema::Set(set) => format!(
            "set(default={:?};{})",
            set.default_key,
            set.nodes
                .iter()
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn project_node(node: &SchemaNode) -> String {
    let mut out = match &node.shape {
        dyn_rt_json_schema::SchemaShape::Scalar => format!("scalar({})", node.type_info.type_name),
        dyn_rt_json_schema::SchemaShape::Any => "any".to_string(),
        dyn_rt_json_schema::SchemaShape::Struct { fields } => format!(
            "struct({})",
            fields
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
                .join(",")
        ),
        dyn_rt_json_schema::SchemaShape::Map {
            entries,
            key,
            value,
        } => format!(
            "map(entries={};key={};value={})",
            entries.len(),
            key.is_some(),
            value.is_some()
        ),
        dyn_rt_json_schema::SchemaShape::Sequence { elements, items } => format!(
            "sequence(elements={};items={})",
            elements.is_some(),
            items.len()
        ),
        dyn_rt_json_schema::SchemaShape::Indirection { .. } => "indirection".to_string(),
        dyn_rt_json_schema::SchemaShape::Enum { variants } => format!(
            "enum({})",
            variants
                .iter()
                .map(|(name, payload)| format!("{}:{}", name, payload.is_some()))
                .collect::<Vec<_>>()
                .join(",")
        ),
    };
    if node.nilable || node.children_must_be_valid {
        out.push_str(&format!(
            " [nilable={},must={}]",
            node.nilable, node.children_must_be_valid
        ));
    }
    out
}

struct DomainCase {
    meta: TestMetadata,
    schema: Schema,
    expected_projection: String,
    expected_kind: Option<ReflectKind>,
}

macro_rules! domain_case {
    ($schema:expr, $projection:expr, $kind:expr $(, test_title = $title:expr)?) => {
        DomainCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            schema: $schema,
            expected_projection: $projection.to_string(),
            expected_kind: $kind,
        }
    };
}

#[test]
fn schema_model_cases() {
    let test_data: Vec<_> = vec![
        domain_case!(
            Schema::Node(SchemaNode::scalar::<String>()),
            "scalar(alloc::string::String)",
            Some(ReflectKind::Scalar),
            test_title = "scalar node for String"
        ),
        domain_case!(
            Schema::Node(SchemaNode::json_scalar("string")),
            "scalar(string)",
            Some(ReflectKind::Scalar),
            test_title = "json scalar node carries the Value variant name"
        ),
        domain_case!(
            Schema::Node(SchemaNode::any()),
            "any",
            Some(ReflectKind::Unknown),
            test_title = "any node"
        ),
        domain_case!(
            Schema::Node(user_schema()),
            "struct(ID,Name,Email)",
            Some(ReflectKind::Struct),
            test_title = "user schema is a struct with renamed fields"
        ),
        domain_case!(
            Schema::Node(json_map_schema()),
            "map(entries=0;key=true;value=true)",
            Some(ReflectKind::Map),
            test_title = "json map schema has key and value schemas"
        ),
        domain_case!(
            Schema::Node(list_of_products_schema()),
            "sequence(elements=true;items=0)",
            Some(ReflectKind::Sequence),
            test_title = "list of products is a sequence with element schema"
        ),
        domain_case!(
            Schema::Node(SchemaNode::indirection_node::<Box<Address>>(Schema::Node(
                address_schema()
            ))),
            "indirection",
            Some(ReflectKind::Indirection),
            test_title = "indirection node wraps the inner schema"
        ),
        domain_case!(
            match &Schema::Node(address_schema()) {
                Schema::Node(node) => match &node.shape {
                    dyn_rt_json_schema::SchemaShape::Struct { fields } => {
                        fields
                            .iter()
                            .find(|(n, _)| n == "ZipCode")
                            .unwrap()
                            .1
                            .clone()
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            "enum(Some:true,None:false) [nilable=true,must=false]",
            Some(ReflectKind::Enum),
            test_title = "Option<String> field is an enum node with Some payload"
        ),
        domain_case!(
            shape_schema(),
            "set(default=Some(\"default\");Circle,Square)",
            None,
            test_title = "shape schema is a set with default key and two nodes"
        ),
        domain_case!(
            Schema::Node(
                SchemaNode::scalar::<i64>()
                    .nilable()
                    .children_must_be_valid()
            ),
            "scalar(i64) [nilable=true,must=true]",
            Some(ReflectKind::Scalar),
            test_title = "nilable/children_must_be_valid flags are set on the node"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let got_projection = project(&case.schema);
        if got_projection != case.expected_projection {
            failures.push(format!(
                "{title} || projection: expected {:?}, got {:?}",
                case.expected_projection, got_projection
            ));
        }
        if let Some(expected_kind) = case.expected_kind {
            let Schema::Node(node) = &case.schema else {
                failures.push(format!("{title} || kind: expected node, got set"));
                continue;
            };
            if node.kind() != expected_kind {
                failures.push(format!(
                    "{title} || kind: expected {:?}, got {:?}",
                    expected_kind,
                    node.kind()
                ));
            }
        }
    }
    report_failures(failures, test_data.len());
}
