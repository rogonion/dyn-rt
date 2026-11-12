//! Inline tests for the `Unknown` kind (ADR 0004 tier 3): a custom type that
//! implements only the base `Reflect` trait is not JSON-expressible, so it
//! cannot live in `tests/shared/` — this is the explicit reason for an
//! inline test.

use std::any::Any;

use dyn_rt_reflect::{
    Reflect, ReflectError, ReflectKind, ReflectRef, ReflectRefMut, as_enum, as_indirection, as_map,
    as_scalar, as_sequence, as_struct, as_unknown, as_unknown_mut, children, kind_of,
};

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

    // Unknowns have no children and clone/apply through the base trait.
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
}
