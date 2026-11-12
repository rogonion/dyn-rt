//! Adapter suite for `src/adapters/unknown.rs`: base-Reflect-only types route
//! to the Unknown kind.

use std::any::Any;
use std::time::Duration;

use crate::common::error_kind;
use dyn_rt_reflect::{
    Reflect, ReflectError, ReflectKind, ReflectRef, ReflectRefMut, as_enum, as_indirection, as_map,
    as_scalar, as_sequence, as_struct, as_unknown, as_unknown_mut, children, kind_of,
};
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

struct UnknownCase {
    meta: TestMetadata,
    value: Duration,
}

macro_rules! unknown_case {
    ($value:expr $(, test_title = $title:expr)?) => {
        UnknownCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            value: $value,
        }
    };
}

#[test]
fn duration_is_unknown() {
    let test_data: Vec<_> = vec![unknown_case!(
        Duration::from_secs(5),
        test_title = "Duration routes to Unknown"
    )];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let reflected: &dyn Reflect = &case.value;
        if kind_of(reflected) != ReflectKind::Unknown {
            failures.push(format!(
                "{title} || kind: expected Unknown, got {:?}",
                kind_of(reflected)
            ));
        }
        if as_unknown(reflected).is_none() {
            failures.push(format!("{title} || as_unknown returned None"));
        }
        if !children(reflected).is_empty() {
            failures.push(format!("{title} || unknown has children"));
        }
    }
    report_failures(failures, test_data.len());
}

// ---------------------------------------------------------------------------
// Inline tests for the Unknown fallback on a custom type (ADR 0004 tier 3): a
// custom type implementing only the base `Reflect` trait is not expressible
// as a plain in-code datum for the shared suites — explicit reason for inline.
// ---------------------------------------------------------------------------

/// A custom type with no shape trait — the `Unknown` fallback case.
struct Foreign {
    label: String,
}

impl Reflect for Foreign {
    fn reflect_type_name(&self) -> &'static str {
        "Foreign"
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Unknown(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Unknown(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(Foreign {
            label: self.label.clone(),
        })
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        let source = value
            .reflect_as_any()
            .downcast_ref::<Foreign>()
            .ok_or_else(|| ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: "Foreign".to_string(),
                source: value.reflect_type_name().to_string(),
            })?;
        self.label = source.label.clone();
        Ok(())
    }

    fn reflect_as_any(&self) -> &dyn Any {
        self
    }

    fn reflect_as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn reflect_into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn reflect_kind() -> ReflectKind {
        ReflectKind::Unknown
    }
}

#[test]
fn unknown_kind_routes_to_base_reflect_only() {
    let value = Foreign {
        label: "x".to_string(),
    };
    let value: &dyn Reflect = &value;

    assert_eq!(kind_of(value), ReflectKind::Unknown);
    assert_eq!(Foreign::reflect_kind(), ReflectKind::Unknown);
    assert_eq!(value.reflect_ref().kind(), ReflectKind::Unknown);

    // The unknown downcast works; every shape downcast is None.
    assert!(as_unknown(value).is_some());
    assert!(as_scalar(value).is_none());
    assert!(as_sequence(value).is_none());
    assert!(as_map(value).is_none());
    assert!(as_struct(value).is_none());
    assert!(as_enum(value).is_none());
    assert!(as_indirection(value).is_none());

    // Unknowns have no children and clone through the base trait.
    assert!(children(value).is_empty());
    let cloned = value.reflect_clone();
    assert_eq!(kind_of(cloned.as_ref()), ReflectKind::Unknown);
    assert_eq!(cloned.reflect_type_name(), "Foreign");
}

#[test]
fn unknown_kind_supports_mut_routing_and_apply() {
    let mut value = Foreign {
        label: "before".to_string(),
    };
    let source = Foreign {
        label: "after".to_string(),
    };
    let value: &mut dyn Reflect = &mut value;

    assert!(matches!(value.reflect_ref_mut(), ReflectRefMut::Unknown(_)));
    assert!(as_unknown_mut(value).is_some());
    value.reflect_apply(&source).unwrap();
    let concrete = value.reflect_as_any().downcast_ref::<Foreign>().unwrap();
    assert_eq!(concrete.label, "after");

    // apply from a non-Foreign type is a ShapeMismatch, not a panic
    let other = 1i64;
    let other: &dyn Reflect = &other;
    let result = value.reflect_apply(other);
    assert_eq!(error_kind(&result.unwrap_err()), "ShapeMismatch");
}
