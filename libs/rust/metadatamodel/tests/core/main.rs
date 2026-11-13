//! Data-driven suites for the metadatamodel `core` subdomain (ADR 0004/0009):
//! src/core/domain.rs + src/core/application.rs — property getters, the
//! templated `group:path_key` post-processing (schema path vs data path),
//! and the strict cardinality rule.

use dyn_rt_metadatamodel::core::application::{
    cardinality_of, check_value_cardinality, data_path, fields_of, name_of, path_key_of, prop,
    props_of, schema_path,
};
use dyn_rt_metadatamodel::core::domain::{Cardinality, FIELDS_KEY, MetadataModel, props};
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// template path cases
// ---------------------------------------------------------------------------

enum TemplateOp {
    SchemaPath {
        indices: Vec<usize>,
    },
    DataPath {
        indices: Vec<usize>,
        cardinality: Cardinality,
    },
}

struct TemplateCase {
    test_title: Option<String>,
    template: &'static str,
    op: TemplateOp,
    expected: Option<String>,
    expect_error: Option<&'static str>,
}

macro_rules! template_case {
    // note: this arm must come first — `expect_error = "..."` also parses as
    // an assignment expression and would be captured by the $expected arm
    ($template:expr, $op:expr, expect_error = $err:expr $(, test_title = $title:expr)?) => {
        TemplateCase {
            test_title: None $(.or(Some($title.to_string())))?,
            template: $template,
            op: $op,
            expected: None,
            expect_error: Some($err),
        }
    };
    ($template:expr, $op:expr, $expected:expr $(, test_title = $title:expr)?) => {
        TemplateCase {
            test_title: None $(.or(Some($title.to_string())))?,
            template: $template,
            op: $op,
            expected: Some($expected.to_string()),
            expect_error: None,
        }
    };
}

#[test]
fn template_cases() {
    let test_data: Vec<TemplateCase> = vec![
        // schema path: every [*] replaced by an index, nothing trimmed
        template_case!(
            "$['group:fields'][*].Bio[*]",
            TemplateOp::SchemaPath {
                indices: vec![0, 0]
            },
            "$['group:fields'][0].Bio[0]",
            test_title = "schema path of a field substitutes all wildcards"
        ),
        template_case!(
            "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
            TemplateOp::SchemaPath {
                indices: vec![0, 1, 0, 2]
            },
            "$['group:fields'][0].Site[1]['group:fields'][0].Country[2]",
            test_title = "schema path of a nested group field"
        ),
        // data path, array cardinality: group:fields + its [*] trimmed,
        // remaining [*] indexed
        template_case!(
            "$['group:fields'][*].Bio[*]",
            TemplateOp::DataPath {
                indices: vec![2],
                cardinality: Cardinality::Array
            },
            "$.Bio[2]",
            test_title = "data path of an array field trims group:fields and indexes the value"
        ),
        // data path, single cardinality: trailing [*] trimmed, no index
        template_case!(
            "$['group:fields'][*].Bio[*]",
            TemplateOp::DataPath {
                indices: vec![],
                cardinality: Cardinality::Single
            },
            "$.Bio",
            test_title = "data path of a single field trims the trailing [*]"
        ),
        template_case!(
            "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
            TemplateOp::DataPath {
                indices: vec![1, 2],
                cardinality: Cardinality::Array
            },
            "$.Site[1].Country[2]",
            test_title = "data path of a nested array field keeps group entry and value indices"
        ),
        template_case!(
            "$",
            TemplateOp::DataPath {
                indices: vec![],
                cardinality: Cardinality::Array
            },
            "$",
            test_title = "root template data path is the root"
        ),
        template_case!(
            "$['group:fields'][*].Bio[*]",
            TemplateOp::SchemaPath { indices: vec![0] },
            expect_error = "path template invalid",
            test_title = "schema path errors when wildcards outnumber indices"
        ),
        template_case!(
            "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
            TemplateOp::DataPath {
                indices: vec![1],
                cardinality: Cardinality::Array
            },
            expect_error = "path template invalid",
            test_title = "data path errors when a data wildcard has no index"
        ),
    ];

    let mut report = TestReport::new(
        "template_cases",
        "group:path_key template post-processing: schema_path / data_path (src/core/application.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let result = match &case.op {
            TemplateOp::SchemaPath { indices } => schema_path(case.template, indices),
            TemplateOp::DataPath {
                indices,
                cardinality,
            } => data_path(case.template, indices, *cardinality),
        };
        match (&case.expect_error, result) {
            (Some(fragment), Err(err)) => {
                if !err.to_string().contains(fragment) {
                    report.fail_case(format!("expected error containing {fragment:?}, got {err}"));
                }
            }
            (Some(fragment), Ok(got)) => {
                report.fail_case(format!(
                    "expected error containing {fragment:?}, got {got:?}"
                ));
            }
            (None, Err(err)) => report.fail_case(format!("unexpected error: {err}")),
            (None, Ok(got)) => {
                if Some(&got) != case.expected.as_ref() {
                    report.fail_case(format!(
                        "template {} || expected {:?}, got {:?}",
                        case.template, case.expected, got
                    ));
                }
            }
        }
    }
    report.finish();
}

// ---------------------------------------------------------------------------
// model property cases
// ---------------------------------------------------------------------------

/// A small metadata model: root group with fields SourceId (single) and
/// Bio (array, default), and a nested group Site with field Country.
fn sample_model() -> MetadataModel {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Vector Data")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                (
                    "SourceId",
                    Value::from(vec![Value::object([
                        (
                            props::GROUP_PATH_KEY,
                            Value::from("$['group:fields'][*].SourceId[*]"),
                        ),
                        (props::GROUP_NAME, Value::from("source id")),
                        (props::FIELD_DATA_TYPE, Value::from("Number")),
                        (props::FIELD_CARDINALITY, Value::from("single")),
                    ])]),
                ),
                (
                    "Bio",
                    Value::from(vec![Value::object([
                        (
                            props::GROUP_PATH_KEY,
                            Value::from("$['group:fields'][*].Bio[*]"),
                        ),
                        (props::FIELD_DATA_TYPE, Value::from("Boolean")),
                    ])]),
                ),
                (
                    "Site",
                    Value::from(vec![Value::object([
                        (
                            props::GROUP_PATH_KEY,
                            Value::from("$['group:fields'][*].Site[*]"),
                        ),
                        (props::GROUP_CARDINALITY, Value::from("array")),
                        (
                            FIELDS_KEY,
                            Value::from(vec![Value::object([(
                                "Country",
                                Value::from(vec![Value::object([
                                    (
                                        props::GROUP_PATH_KEY,
                                        Value::from(
                                            "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
                                        ),
                                    ),
                                    (props::FIELD_DATA_TYPE, Value::from("Text")),
                                ])]),
                            )])]),
                        ),
                    ])]),
                ),
            ])]),
        ),
    ])
}

enum ModelOp {
    PathKeyOf {
        expected: &'static str,
    },
    NameOf {
        default: &'static str,
        expected: String,
    },
    FieldsContain {
        field: &'static str,
    },
    PropsDataType {
        field: &'static str,
        expected: &'static str,
    },
    CardinalityOf {
        field: &'static str,
        expected: Cardinality,
    },
    CheckCardinality {
        value: Value,
        cardinality: Cardinality,
        expect_error: Option<&'static str>,
    },
}

struct ModelCase {
    test_title: Option<String>,
    op: ModelOp,
}

macro_rules! model_case {
    ($op:expr $(, test_title = $title:expr)?) => {
        ModelCase {
            test_title: None $(.or(Some($title.to_string())))?,
            op: $op,
        }
    };
}

#[test]
fn model_cases() {
    let test_data: Vec<ModelCase> = vec![
        model_case!(
            ModelOp::PathKeyOf { expected: "$" },
            test_title = "root group path key"
        ),
        model_case!(
            ModelOp::NameOf {
                default: "",
                expected: "Vector Data".to_string()
            },
            test_title = "name from group:name"
        ),
        model_case!(
            ModelOp::FieldsContain { field: "Site" },
            test_title = "fields_of exposes the nested group"
        ),
        model_case!(
            ModelOp::PropsDataType {
                field: "SourceId",
                expected: "Number"
            },
            test_title = "props_of unwraps the field props array"
        ),
        model_case!(
            ModelOp::CardinalityOf {
                field: "SourceId",
                expected: Cardinality::Single
            },
            test_title = "explicit single cardinality"
        ),
        model_case!(
            ModelOp::CardinalityOf {
                field: "Bio",
                expected: Cardinality::Array
            },
            test_title = "cardinality defaults to array"
        ),
        model_case!(
            ModelOp::CheckCardinality {
                value: Value::from(vec![Value::from(1i64)]),
                cardinality: Cardinality::Array,
                expect_error: None,
            },
            test_title = "array value satisfies array cardinality"
        ),
        model_case!(
            ModelOp::CheckCardinality {
                value: Value::from(1i64),
                cardinality: Cardinality::Array,
                expect_error: Some("cardinality violation"),
            },
            test_title = "single value violates array cardinality (strict)"
        ),
        model_case!(
            ModelOp::CheckCardinality {
                value: Value::from(vec![Value::from(1i64)]),
                cardinality: Cardinality::Single,
                expect_error: Some("cardinality violation"),
            },
            test_title = "array value violates single cardinality (strict)"
        ),
        model_case!(
            ModelOp::CheckCardinality {
                value: Value::from("text"),
                cardinality: Cardinality::Single,
                expect_error: None,
            },
            test_title = "single value satisfies single cardinality"
        ),
    ];

    let mut report = TestReport::new(
        "model_cases",
        "model property getters + strict cardinality (src/core/application.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let model = sample_model();
        match &case.op {
            ModelOp::PathKeyOf { expected } => match path_key_of(&model) {
                Ok(got) if got == *expected => {}
                Ok(got) => report.fail_case(format!("expected {expected:?}, got {got:?}")),
                Err(err) => report.fail_case(format!("unexpected error: {err}")),
            },
            ModelOp::NameOf { default, expected } => {
                let got = name_of(&model, default);
                if got != *expected {
                    report.fail_case(format!("expected {expected:?}, got {got:?}"));
                }
            }
            ModelOp::FieldsContain { field } => {
                let fields = fields_of(&model).expect("fields_of");
                if prop(fields, field).is_none() {
                    report.fail_case(format!("fields object has no {field:?} entry"));
                }
            }
            ModelOp::PropsDataType { field, expected } => {
                let fields = fields_of(&model).expect("fields_of");
                let entry = prop(fields, field).expect("field entry");
                let props = props_of(entry).expect("props_of");
                let got = prop(props, props::FIELD_DATA_TYPE)
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                if got != *expected {
                    report.fail_case(format!("expected {expected:?}, got {got:?}"));
                }
            }
            ModelOp::CardinalityOf { field, expected } => {
                let fields = fields_of(&model).expect("fields_of");
                let entry = prop(fields, field).expect("field entry");
                let props = props_of(entry).expect("props_of");
                let got = cardinality_of(props);
                if got != *expected {
                    report.fail_case(format!("expected {expected:?}, got {got:?}"));
                }
            }
            ModelOp::CheckCardinality {
                value,
                cardinality,
                expect_error,
            } => {
                let reflected: &dyn Reflect = value;
                let result = check_value_cardinality(reflected, *cardinality);
                match (expect_error, result) {
                    (Some(fragment), Err(err)) => {
                        if !err.to_string().contains(fragment) {
                            report.fail_case(format!(
                                "expected error containing {fragment:?}, got {err}"
                            ));
                        }
                    }
                    (Some(fragment), Ok(())) => {
                        report.fail_case(format!("expected error containing {fragment:?}, got Ok"));
                    }
                    (None, Err(err)) => report.fail_case(format!("unexpected error: {err}")),
                    (None, Ok(())) => {}
                }
            }
        }
    }
    report.finish();
}
