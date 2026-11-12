//! Adapter suite for `src/adapters/enum_shape.rs`: Option<T> as an enum.

use crate::common::{error_kind, value_of};
use dyn_rt_reflect::{Reflect, ReflectRefMut, as_enum};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

#[derive(Clone)]
enum Subject {
    Some(Value),
    None,
}

impl Subject {
    fn into_reflected(self) -> Box<dyn Reflect> {
        match self {
            Subject::Some(v) => Box::new(Some(v)),
            Subject::None => Box::new(None::<Value>),
        }
    }
}

enum EnumOp {
    Variant(&'static str),
    Payload(Value),
    SetVariant {
        variant: &'static str,
        expected_none: bool,
        expect_error: Option<&'static str>,
    },
}

struct EnumCase {
    meta: TestMetadata,
    subject: Subject,
    op: EnumOp,
}

macro_rules! enum_case {
    ($subject:expr, $op:expr $(, test_title = $title:expr)?) => {
        EnumCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            subject: $subject,
            op: $op,
        }
    };
}

#[test]
fn option_enum_ops() {
    let test_data: Vec<_> = vec![
        enum_case!(
            Subject::Some(5i64.into()),
            EnumOp::Variant("Some"),
            test_title = "option some reports Some variant"
        ),
        enum_case!(
            Subject::None,
            EnumOp::Variant("None"),
            test_title = "option none reports None variant"
        ),
        enum_case!(
            Subject::Some(5i64.into()),
            EnumOp::Payload(5i64.into()),
            test_title = "option some payload readable"
        ),
        enum_case!(
            Subject::Some(5i64.into()),
            EnumOp::SetVariant {
                variant: "None",
                expected_none: true,
                expect_error: None
            },
            test_title = "option set_variant to None"
        ),
        enum_case!(
            Subject::Some(5i64.into()),
            EnumOp::SetVariant {
                variant: "Maybe",
                expected_none: false,
                expect_error: Some("UnknownVariant")
            },
            test_title = "option set_variant unknown fails"
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let mut subject = case.subject.clone().into_reflected();

        match &case.op {
            EnumOp::Variant(expected) => {
                let enumeration = as_enum(subject.as_ref()).expect("enum expected");
                if enumeration.reflect_active_variant() != *expected {
                    failures.push(format!(
                        "{title} || variant: expected {expected:?}, got {:?}",
                        enumeration.reflect_active_variant()
                    ));
                }
            }
            EnumOp::Payload(expected) => {
                let enumeration = as_enum(subject.as_ref()).expect("enum expected");
                let payload = enumeration
                    .reflect_active_variant_payload()
                    .map(value_of)
                    .expect("payload expected");
                if payload != expected {
                    failures.push(format!(
                        "{title} || payload: expected {expected:?}, got {payload:?}"
                    ));
                }
            }
            EnumOp::SetVariant {
                variant,
                expected_none,
                expect_error,
            } => {
                let result = match subject.reflect_ref_mut() {
                    ReflectRefMut::Enum(e) => e.reflect_set_variant(variant, None),
                    _ => {
                        failures.push(format!("{title} || set_variant: expected enum"));
                        continue;
                    }
                };
                match expect_error {
                    Some(expected_error) => match &result {
                        Err(err) if error_kind(err) == *expected_error => {}
                        other => failures.push(format!("{title} || set_variant {variant:?}: expected error {expected_error}, got {other:?}")),
                    },
                    None => match result {
                        Ok(()) => {
                            let enumeration = as_enum(subject.as_ref()).expect("enum");
                            if enumeration.reflect_active_variant() != *variant {
                                failures.push(format!("{title} || set_variant {variant:?}: variant is {:?}", enumeration.reflect_active_variant()));
                            }
                            if *expected_none && enumeration.reflect_active_variant_payload().is_some() {
                                failures.push(format!("{title} || set_variant {variant:?}: expected no payload"));
                            }
                        }
                        Err(err) => failures.push(format!("{title} || set_variant {variant:?}: unexpected error {}", error_kind(&err))),
                    },
                }
            }
        }
    }
    report_failures(failures, test_data.len());
}
