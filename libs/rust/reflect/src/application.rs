//! Reflect application: thin semantic helpers over the domain traits.
//! Imports domain only.

use crate::domain::{Kind, Reflect, ReflectMap, ReflectRef, ReflectRefMut, ReflectSequence};

/// The kind of a reflectable value.
pub fn kind_of(value: &dyn Reflect) -> Kind {
    value.reflect_ref().kind()
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

/// The children of a container value: map values in key order, or sequence
/// elements in order. Scalars and indirections have no children here.
/// Used by recursive-descent traversal.
pub fn children(value: &dyn Reflect) -> Vec<&dyn Reflect> {
    match value.reflect_ref() {
        ReflectRef::Map(map) => map.reflect_iter().into_iter().map(|(_, v)| v).collect(),
        ReflectRef::Sequence(sequence) => sequence.reflect_iter(),
        _ => Vec::new(),
    }
}
