//! Data-driven suites for the reflect DOMAIN layer (ADR 0004): ReflectKind,
//! TypeInfo/type_info. One custom loop per suite, every case titled.

mod common;

use dyn_rt_reflect::{ReflectKind, TypeInfo, type_info};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

struct KindNameCase {
    meta: TestMetadata,
    kind: ReflectKind,
    expected: &'static str,
}

macro_rules! kind_name_case {
    ($kind:expr, $expected:literal $(, test_title = $title:expr)?) => {
        KindNameCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            kind: $kind,
            expected: $expected,
        }
    };
}

#[test]
fn reflect_kind_display() {
    let test_data: Vec<_> = vec![
        kind_name_case!(ReflectKind::Scalar, "scalar", test_title = "scalar renders"),
        kind_name_case!(
            ReflectKind::Sequence,
            "sequence",
            test_title = "sequence renders"
        ),
        kind_name_case!(ReflectKind::Map, "map", test_title = "map renders"),
        kind_name_case!(ReflectKind::Struct, "struct", test_title = "struct renders"),
        kind_name_case!(ReflectKind::Enum, "enum", test_title = "enum renders"),
        kind_name_case!(
            ReflectKind::Indirection,
            "indirection",
            test_title = "indirection renders"
        ),
        kind_name_case!(
            ReflectKind::Unknown,
            "unknown",
            test_title = "unknown renders"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let got = case.kind.to_string();
        if got != case.expected {
            failures.push(format!(
                "{} || expected {:?}, got {:?}",
                new_test_title!(test_index + 1, case.meta.test_title),
                case.expected,
                got,
            ));
        }
    }
    report_failures(failures, test_data.len());
}

struct TypeInfoCase {
    meta: TestMetadata,
    info: TypeInfo,
    expected_kind: ReflectKind,
    expected_type_name: Option<&'static str>,
    constructible: bool,
}

macro_rules! type_info_case {
    ($info:expr, $expected_kind:expr, $expected_type_name:expr, $constructible:expr $(, test_title = $title:expr)?) => {
        TypeInfoCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            info: $info,
            expected_kind: $expected_kind,
            expected_type_name: $expected_type_name,
            constructible: $constructible,
        }
    };
}

#[test]
fn type_info_table() {
    let test_data: Vec<_> = vec![
        type_info_case!(
            TypeInfo::of::<bool>(),
            ReflectKind::Scalar,
            Some("bool"),
            false,
            test_title = "bool type info"
        ),
        type_info_case!(
            TypeInfo::of::<i64>(),
            ReflectKind::Scalar,
            Some("i64"),
            false,
            test_title = "i64 type info"
        ),
        type_info_case!(
            TypeInfo::of::<String>(),
            ReflectKind::Scalar,
            Some("alloc::string::String"),
            false,
            test_title = "String type info"
        ),
        type_info_case!(
            TypeInfo::of::<Vec<i64>>(),
            ReflectKind::Sequence,
            Some("alloc::vec::Vec<i64>"),
            false,
            test_title = "Vec<i64> type info"
        ),
        // Value is a nominal enum at the type level (instances route by
        // variant); of() carries no constructor...
        type_info_case!(
            TypeInfo::of::<Value>(),
            ReflectKind::Enum,
            None,
            false,
            test_title = "Value type info via of() is not constructible"
        ),
        // ...but of_constructible() carries the ReflectNew constructor.
        type_info_case!(
            TypeInfo::of_constructible::<Value>(),
            ReflectKind::Enum,
            None,
            true,
            test_title = "Value type info via of_constructible() constructs"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        if case.info.kind != case.expected_kind {
            failures.push(format!(
                "{title} || kind: expected {:?}, got {:?}",
                case.expected_kind, case.info.kind,
            ));
        }
        if let Some(expected_name) = case.expected_type_name
            && case.info.type_name != expected_name
        {
            failures.push(format!(
                "{title} || type_name: expected {expected_name:?}, got {:?}",
                case.info.type_name,
            ));
        }
        if case.info.construct().is_some() != case.constructible {
            failures.push(format!(
                "{title} || constructible: expected {}, got {}",
                case.constructible,
                case.info.construct().is_some(),
            ));
        }
    }
    report_failures(failures, test_data.len());

    // type_info is cached per monomorphization: same pointer on repeat calls.
    // Not data — a direct identity assertion (ADR 0004 tier 3, inline reason).
    assert!(std::ptr::eq(type_info::<i64>(), type_info::<i64>()));
}
