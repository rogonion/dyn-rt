//! Reflect application: thin semantic helpers over the domain traits.
//! Imports domain only — one downcast helper pair per [`ReflectKind`].

use crate::domain::{
    Reflect, ReflectEnum, ReflectIndirection, ReflectKind, ReflectMap, ReflectRef, ReflectRefMut,
    ReflectSequence, ReflectStruct,
};
use crate::error::ReflectError;

/// The kind of a reflectable value.
pub fn kind_of(value: &dyn Reflect) -> ReflectKind {
    value.reflect_ref().kind()
}

/// The children of a container value: map values in key order, or sequence
/// elements in order. Scalars, indirections and unknowns have no children
/// here. Used by recursive-descent traversal.
pub fn children(value: &dyn Reflect) -> Vec<&dyn Reflect> {
    match value.reflect_ref() {
        ReflectRef::Map(map) => map.reflect_iter().map(|(_, v)| v).collect(),
        ReflectRef::Sequence(sequence) => sequence.reflect_iter().collect(),
        _ => Vec::new(),
    }
}

/// Downcast to the scalar view, if the value is one.
pub fn as_scalar(value: &dyn Reflect) -> Option<&dyn Reflect> {
    match value.reflect_ref() {
        ReflectRef::Scalar(scalar) => Some(scalar),
        _ => None,
    }
}

/// Downcast to the scalar view mutably, if the value is one.
pub fn as_scalar_mut(value: &mut dyn Reflect) -> Option<&mut dyn Reflect> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Scalar(scalar) => Some(scalar),
        _ => None,
    }
}

/// Downcast to the sequence shape, if the value is one.
pub fn as_sequence(value: &dyn Reflect) -> Option<&dyn ReflectSequence> {
    match value.reflect_ref() {
        ReflectRef::Sequence(sequence) => Some(sequence),
        _ => None,
    }
}

/// Downcast to the sequence shape mutably, if the value is one.
pub fn as_sequence_mut(value: &mut dyn Reflect) -> Option<&mut dyn ReflectSequence> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Sequence(sequence) => Some(sequence),
        _ => None,
    }
}

/// Downcast to the map shape, if the value is one.
pub fn as_map(value: &dyn Reflect) -> Option<&dyn ReflectMap> {
    match value.reflect_ref() {
        ReflectRef::Map(map) => Some(map),
        _ => None,
    }
}

/// Downcast to the map shape mutably, if the value is one.
pub fn as_map_mut(value: &mut dyn Reflect) -> Option<&mut dyn ReflectMap> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Map(map) => Some(map),
        _ => None,
    }
}

/// Downcast to the struct shape, if the value is one.
pub fn as_struct(value: &dyn Reflect) -> Option<&dyn ReflectStruct> {
    match value.reflect_ref() {
        ReflectRef::Struct(struct_) => Some(struct_),
        _ => None,
    }
}

/// Downcast to the struct shape mutably, if the value is one.
pub fn as_struct_mut(value: &mut dyn Reflect) -> Option<&mut dyn ReflectStruct> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Struct(struct_) => Some(struct_),
        _ => None,
    }
}

/// Downcast to the enum shape, if the value is one.
pub fn as_enum(value: &dyn Reflect) -> Option<&dyn ReflectEnum> {
    match value.reflect_ref() {
        ReflectRef::Enum(enumeration) => Some(enumeration),
        _ => None,
    }
}

/// Downcast to the enum shape mutably, if the value is one.
pub fn as_enum_mut(value: &mut dyn Reflect) -> Option<&mut dyn ReflectEnum> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Enum(enumeration) => Some(enumeration),
        _ => None,
    }
}

/// Downcast to the indirection shape, if the value is one.
pub fn as_indirection(value: &dyn Reflect) -> Option<&dyn ReflectIndirection> {
    match value.reflect_ref() {
        ReflectRef::Indirection(indirection) => Some(indirection),
        _ => None,
    }
}

/// Downcast to the indirection shape mutably, if the value is one.
pub fn as_indirection_mut(value: &mut dyn Reflect) -> Option<&mut dyn ReflectIndirection> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Indirection(indirection) => Some(indirection),
        _ => None,
    }
}

/// Downcast to the unknown view, if the value has no shape trait.
pub fn as_unknown(value: &dyn Reflect) -> Option<&dyn Reflect> {
    match value.reflect_ref() {
        ReflectRef::Unknown(unknown) => Some(unknown),
        _ => None,
    }
}

/// Downcast to the unknown view mutably, if the value has no shape trait.
pub fn as_unknown_mut(value: &mut dyn Reflect) -> Option<&mut dyn Reflect> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Unknown(unknown) => Some(unknown),
        _ => None,
    }
}

/// Scoped read of an indirection, callback-free at the call site.
pub fn reflect_with_inner<R>(
    value: &dyn Reflect,
    on_inner: impl FnOnce(&dyn Reflect) -> R,
) -> Result<R, ReflectError> {
    let type_name = value.reflect_type_name().to_string();
    match value.reflect_ref() {
        ReflectRef::Indirection(indirection) => {
            let mut on_inner = Some(on_inner);
            let mut result: Option<Result<R, ReflectError>> = None;
            indirection.reflect_get_inner(&mut |inner| {
                result = Some(match inner {
                    Ok(inner) => Ok(on_inner.take().expect("callback invoked once")(inner)),
                    Err(error) => Err(error),
                });
            });
            result.unwrap_or_else(|| Err(ReflectError::InnerNotProvided { type_name }))
        }
        _ => Err(ReflectError::UnsupportedOperation {
            operation: "reflect_with_inner",
            type_name,
        }),
    }
}

/// Scoped mutable access to an indirection, callback-free at the call site.
pub fn reflect_with_inner_mut<R>(
    value: &mut dyn Reflect,
    on_inner: impl FnOnce(&mut dyn Reflect) -> R,
) -> Result<R, ReflectError> {
    let type_name = value.reflect_type_name().to_string();
    match value.reflect_ref_mut() {
        ReflectRefMut::Indirection(indirection) => {
            let mut on_inner = Some(on_inner);
            let mut result: Option<Result<R, ReflectError>> = None;
            indirection.reflect_get_inner_mut(&mut |inner| {
                result = Some(match inner {
                    Ok(inner) => Ok(on_inner.take().expect("callback invoked once")(inner)),
                    Err(error) => Err(error),
                });
            });
            result.unwrap_or_else(|| Err(ReflectError::InnerNotProvided { type_name }))
        }
        _ => Err(ReflectError::UnsupportedOperation {
            operation: "reflect_with_inner_mut",
            type_name,
        }),
    }
}
