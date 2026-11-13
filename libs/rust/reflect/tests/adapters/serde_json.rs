//! Data-driven suite for src/adapters/serde_json.rs (ADR 0004/0009), gated
//! on the `serde-json` feature: serde_json::Value participating in reflection.

use dyn_rt_reflect::{Reflect, ReflectEnum, ReflectKind, as_map, as_sequence, kind_of};
use dyn_rt_test_support::{TestReport, new_test_title};

struct SerdeJsonCase {
    test_title: Option<String>,
    value: serde_json::Value,
    expected_kind: ReflectKind,
    expected_variant: &'static str,
    expected_type_name: &'static str,
}

macro_rules! serde_json_case {
    ($value:expr, $kind:expr, $variant:expr, $type_name:expr $(, test_title = $title:expr)?) => {
        SerdeJsonCase {
test_title: None $(.or(Some($title.to_string())))?,
            value: $value,
            expected_kind: $kind,
            expected_variant: $variant,
            expected_type_name: $type_name,
        }
    };
}

#[test]
fn serde_json_value_cases() {
    let test_data: Vec<_> = vec![
        serde_json_case!(
            serde_json::json!(null),
            ReflectKind::Scalar,
            "Null",
            "null",
            test_title = "null routes to Scalar / Null variant"
        ),
        serde_json_case!(
            serde_json::json!(true),
            ReflectKind::Scalar,
            "Bool",
            "boolean",
            test_title = "bool routes to Scalar / Bool variant with payload"
        ),
        serde_json_case!(
            serde_json::json!(42),
            ReflectKind::Scalar,
            "Number",
            "number",
            test_title = "number routes to Scalar / Number variant"
        ),
        serde_json_case!(
            serde_json::json!("x"),
            ReflectKind::Scalar,
            "String",
            "string",
            test_title = "string routes to Scalar / String variant"
        ),
        serde_json_case!(
            serde_json::json!([1, 2]),
            ReflectKind::Sequence,
            "Array",
            "array",
            test_title = "array routes to Sequence / Array variant"
        ),
        serde_json_case!(
            serde_json::json!({"a": 1}),
            ReflectKind::Map,
            "Object",
            "object",
            test_title = "object routes to Map / Object variant"
        ),
    ];

    let mut report = TestReport::new(
        "serde_json_value_cases",
        "Data-driven suite for src/adapters/serde_json.rs (ADR 0004/0009), gated",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let r: &dyn Reflect = &case.value;
        if kind_of(r) != case.expected_kind {
            report.fail_case(format!(
                "kind: expected {:?}, got {:?}",
                case.expected_kind,
                kind_of(r)
            ));
        }
        if r.reflect_type_name() != case.expected_type_name {
            report.fail_case(format!(
                "type name: expected {:?}, got {:?}",
                case.expected_type_name,
                r.reflect_type_name()
            ));
        }
        let enumeration: &dyn ReflectEnum = &case.value;
        if enumeration.reflect_active_variant() != case.expected_variant {
            report.fail_case(format!(
                "variant: expected {:?}, got {:?}",
                case.expected_variant,
                enumeration.reflect_active_variant()
            ));
        }
    }

    // shape access: map get, sequence get, enum payload
    let object = serde_json::json!({"a": 1});
    let r: &dyn Reflect = &object;
    let map = as_map(r).expect("map");
    let got = map
        .reflect_get(&"a".to_string())
        .map(|v| v.reflect_type_name().to_string());
    if got.as_deref() != Some("number") {
        report.fail(
            "Test case shape access",
            format!("map get 'a': expected Some(\"number\"), got {got:?}"),
        );
    }
    let array = serde_json::json!([10, 20]);
    let r: &dyn Reflect = &array;
    let seq = as_sequence(r).expect("sequence");
    if seq.reflect_length() != 2
        || seq.reflect_get(1).map(|v| v.reflect_type_name()) != Some("number")
    {
        report.fail(
            "Test case shape access",
            format!(
                "sequence len/get: got len={}, [1]={:?}",
                seq.reflect_length(),
                seq.reflect_get(1).map(|v| v.reflect_type_name())
            ),
        );
    }
    let boolean = serde_json::json!(true);
    let payload = ReflectEnum::reflect_active_variant_payload(&boolean).expect("payload");
    if payload.reflect_as_any().downcast_ref::<bool>() != Some(&true) {
        report.fail(
            "Test case shape access",
            format!("Bool payload: got {:?}", payload.reflect_type_name()),
        );
    }

    report.finish();
}
