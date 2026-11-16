//! `are_equal`: recursive structural equality over reflected values (port of
//! go-json's `object.AreEqual`).
//!
//! Sequences, maps, structs and enums are compared element/field/variant-wise
//! (maps by key matching, like go); scalars by concrete type plus scalar
//! value; indirections (`Box`/`Rc`/`Some`) are unwrapped before comparing.
//! Custom per-type equality checkers can be registered by `TypeId`
//! (go's `AreEquals` map keyed by `reflect.Type`).

use std::any::TypeId;
use std::collections::HashMap;

use dyn_rt_reflect::{Reflect, ReflectRef};

/// A custom equality checker for one concrete type, keyed by its [`TypeId`]
/// in the map passed to [`are_equal_with`].
pub type CustomEqualFn = fn(&dyn Reflect, &dyn Reflect) -> bool;

/// Recursively checks whether `left` and `right` are structurally equal.
pub fn are_equal(left: &dyn Reflect, right: &dyn Reflect) -> bool {
    are_equal_with(left, right, None)
}

/// [`are_equal`] with custom per-type equality checkers (go's
/// `WithCustomEquals`).
pub fn are_equal_with(
    left: &dyn Reflect,
    right: &dyn Reflect,
    custom: Option<&HashMap<TypeId, CustomEqualFn>>,
) -> bool {
    equal(left, right, custom)
}

fn equal(
    left: &dyn Reflect,
    right: &dyn Reflect,
    custom: Option<&HashMap<TypeId, CustomEqualFn>>,
) -> bool {
    // unwrap one level of indirection (Box/Rc/Some payload) on both sides,
    // like go unwrapping Ptr/Interface at every level
    let left = super::unwrap(left).unwrap_or(left);
    let right = super::unwrap(right).unwrap_or(right);

    if let Some(custom) = custom
        && let Some(checker) = custom.get(&left.reflect_as_any().type_id())
    {
        return checker(left, right);
    }

    let left_ref = left.reflect_ref();
    let right_ref = right.reflect_ref();
    if left_ref.kind() != right_ref.kind() {
        return false;
    }

    match (left_ref, right_ref) {
        (ReflectRef::Scalar(_), ReflectRef::Scalar(_)) => {
            left.reflect_as_any().type_id() == right.reflect_as_any().type_id()
                && left.reflect_scalar_i64() == right.reflect_scalar_i64()
                && left.reflect_scalar_f64() == right.reflect_scalar_f64()
                && left.reflect_scalar_bool() == right.reflect_scalar_bool()
                && left.reflect_scalar_string() == right.reflect_scalar_string()
        }
        (ReflectRef::Sequence(l), ReflectRef::Sequence(r)) => {
            if l.reflect_length() != r.reflect_length() {
                return false;
            }
            for index in 0..l.reflect_length() {
                let lv = l.reflect_get(index).expect("index within length");
                let rv = r.reflect_get(index).expect("index within length");
                if !equal(lv, rv, custom) {
                    return false;
                }
            }
            true
        }
        (ReflectRef::Map(l), ReflectRef::Map(r)) => {
            if l.reflect_length() != r.reflect_length() {
                return false;
            }
            for (left_key, left_value) in l.reflect_iter() {
                let mut matched = false;
                for (right_key, right_value) in r.reflect_iter() {
                    if equal(left_key, right_key, custom) {
                        if !equal(left_value, right_value, custom) {
                            return false;
                        }
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return false;
                }
            }
            true
        }
        (ReflectRef::Struct(l), ReflectRef::Struct(r)) => {
            let left_infos = l.reflect_field_infos();
            let right_infos = r.reflect_field_infos();
            if left_infos.len() != right_infos.len() {
                return false;
            }
            for (left_info, right_info) in left_infos.iter().zip(right_infos.iter()) {
                if left_info.name != right_info.name {
                    return false;
                }
            }
            for index in 0..left_infos.len() {
                let lv = l.reflect_field_at(index).expect("field within count");
                let rv = r.reflect_field_at(index).expect("field within count");
                if !equal(lv, rv, custom) {
                    return false;
                }
            }
            true
        }
        (ReflectRef::Enum(l), ReflectRef::Enum(r)) => {
            if l.reflect_active_variant() != r.reflect_active_variant() {
                return false;
            }
            match (
                l.reflect_active_variant_payload(),
                r.reflect_active_variant_payload(),
            ) {
                (Some(lp), Some(rp)) => equal(lp, rp, custom),
                (None, None) => true,
                _ => false,
            }
        }
        // indirections that do not expose an inner (RefCell/Mutex/...) and
        // unknown base-only types: same concrete type and scalar form
        (ReflectRef::Indirection(_), ReflectRef::Indirection(_))
        | (ReflectRef::Unknown(_), ReflectRef::Unknown(_)) => {
            left.reflect_as_any().type_id() == right.reflect_as_any().type_id()
                && left.reflect_scalar_string() == right.reflect_scalar_string()
        }
        _ => false,
    }
}
