//! Data-driven suite for src/adapters/deser.rs (ADR 0004/0009): text ->
//! reflect-able serde value (json/yaml/toml), plus the 2-step integration
//! into a typed target via conversion. Gated on the `deser` feature.

use crate::common::*;
use dyn_rt_json_schema::Schema;
use dyn_rt_json_schema::adapters::deser::{from_json_str, from_toml_str, from_yaml_str};
use dyn_rt_json_schema::application::convert;
use dyn_rt_reflect::Reflect;
use dyn_rt_test_support::{TestReport, new_test_title};

enum Format {
    Json,
    Yaml,
    Toml,
}

struct DeserCase {
    test_title: Option<String>,
    format: Format,
    text: &'static str,
    /// Ok(expected debug render of the parsed serde value) or Err(fragment)
    expected: Result<String, &'static str>,
}

macro_rules! deser_case {
    ($format:expr, $text:expr, $expected:expr $(, test_title = $title:expr)?) => {
        DeserCase {
test_title: None $(.or(Some($title.to_string())))?,
            format: $format,
            text: $text,
            expected: $expected,
        }
    };
}

fn parse(case: &DeserCase) -> Result<Box<dyn Reflect>, dyn_rt_json_schema::SchemaError> {
    match case.format {
        Format::Json => from_json_str(case.text),
        Format::Yaml => from_yaml_str(case.text),
        Format::Toml => from_toml_str(case.text),
    }
}

#[test]
fn deser_cases() {
    let user_json = serde_json::json!({"ID": 1, "Name": "amy", "Email": "amy@example.com"});
    let test_data: Vec<_> = vec![
        deser_case!(
            Format::Json,
            r#"{"ID": 1, "Name": "amy", "Email": "amy@example.com"}"#,
            Ok(format!("{user_json:?}")),
            test_title = "json text parses into a reflect-able serde value"
        ),
        deser_case!(
            Format::Yaml,
            "ID: 1\nName: amy\nEmail: amy@example.com\n",
            Ok(format!("{user_json:?}")),
            test_title = "yaml text parses into the same serde value shape"
        ),
        deser_case!(
            Format::Toml,
            "ID = 1\nName = \"amy\"\nEmail = \"amy@example.com\"\n",
            Ok(format!("{user_json:?}")),
            test_title = "toml text parses into the same serde value shape"
        ),
        deser_case!(
            Format::Json,
            "[1, \"two\", true, null]",
            Ok(format!("{:?}", serde_json::json!([1, "two", true, null]))),
            test_title = "json array with mixed scalars"
        ),
        deser_case!(
            Format::Yaml,
            "- 1\n- two\n",
            Ok(format!("{:?}", serde_json::json!([1, "two"]))),
            test_title = "yaml sequence"
        ),
        deser_case!(
            Format::Json,
            "{not json",
            Err("json:"),
            test_title = "invalid json errors DeserializationFailed"
        ),
        deser_case!(
            Format::Toml,
            "not = = toml",
            Err("toml:"),
            test_title = "invalid toml errors DeserializationFailed"
        ),
    ];

    let mut report = TestReport::new(
        "deser_cases",
        "Data-driven suite for src/adapters/deser.rs (ADR 0004/0009): text ->",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let got = parse(case);
        match (&case.expected, &got) {
            (Ok(expected), Ok(value)) => {
                let rendered = format!(
                    "{:?}",
                    value
                        .reflect_as_any()
                        .downcast_ref::<serde_json::Value>()
                        .expect("deser returns serde_json::Value")
                );
                if &rendered != expected {
                    report.fail_case(format!("expected {:?}, got {:?}", expected, rendered));
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
            _ => report.fail_case(format!(
                "expected {:?}, got {}",
                case.expected,
                got.as_ref().map(|_| "Ok(..)").unwrap_or("Err(..)")
            )),
        }
    }

    // 2-step integration: yaml text -> serde value (reflect-able) ->
    // conversion into the schema-dictated struct
    let yaml = "ID: 1\nName: amy\nEmail: amy@example.com\n";
    let parsed = from_yaml_str(yaml).expect("yaml parses");
    let mut target: Box<dyn Reflect> = Box::new(User::default());
    let outcome = convert(&*parsed, &Schema::Node(user_schema()), &mut *target);
    match outcome {
        Ok(_) => {
            let rendered = format!(
                "{:?}",
                target
                    .reflect_as_any()
                    .downcast_ref::<User>()
                    .expect("target is User")
            );
            if rendered != format!("{:?}", sample_user()) {
                report.fail(
                    "Test case integration: yaml -> convert into User",
                    format!(
                        "expected {:?}, got {:?}",
                        format!("{:?}", sample_user()),
                        rendered
                    ),
                );
            }
        }
        Err(err) => report.fail(
            "Test case integration: yaml -> convert into User",
            format!("unexpected error {err:?}"),
        ),
    }

    report.finish();
}
