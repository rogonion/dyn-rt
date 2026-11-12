//! Serde adapter (feature `serde-json`): `impl Reflect` for `serde_json::Value`
//! and its `Number` / `Map` parts, so serde-parsed JSON participates in the
//! reflection engine. This is the only file in the crate that imports
//! `serde_json` (ADR 0002).
//!
//! `serde_json::Value` is an enum (`Null/Bool/Number/String/Array/Object`),
//! so it gets the full `ReflectEnum` shape; instances route by variant for
//! map/sequence/scalar access, mirroring the `shared::Value` adapter.

use std::any::{Any, TypeId, type_name};

use serde_json::{Map, Number, Value};

use super::{into_t, unsupported};
use crate::domain::{
    Reflect, ReflectEnum, ReflectKind, ReflectMap, ReflectNew, ReflectRef, ReflectRefMut,
    ReflectSequence, TypeInfo, VariantInfo, type_info,
};
use crate::error::ReflectError;

impl Reflect for Value {
    fn reflect_type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        match self {
            Value::Object(_) => ReflectRef::Map(self),
            Value::Array(_) => ReflectRef::Sequence(self),
            _ => ReflectRef::Scalar(self),
        }
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        match self {
            Value::Object(_) => ReflectRefMut::Map(self),
            Value::Array(_) => ReflectRefMut::Sequence(self),
            _ => ReflectRefMut::Scalar(self),
        }
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        if value.reflect_as_any().type_id() != TypeId::of::<Value>() {
            return Err(ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: type_name::<Value>().to_string(),
                source: value.reflect_type_name().to_string(),
            });
        }
        let source = value
            .reflect_as_any()
            .downcast_ref::<Value>()
            .expect("type id checked");
        *self = source.clone();
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
        // Nominal enum at the type level; instances route by variant.
        ReflectKind::Enum
    }

    fn reflect_scalar_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n
                .as_i64()
                .or_else(|| n.as_u64().and_then(|v| i64::try_from(v).ok())),
            _ => None,
        }
    }

    fn reflect_scalar_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    fn reflect_scalar_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn reflect_scalar_string(&self) -> Option<String> {
        match self {
            Value::Null => Some("null".to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl ReflectNew for Value {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(Value::Null)
    }
}

/// Extract a string key from an erased reflected key (`String` or
/// `serde_json::Value::String`).
fn key_as_str(key: &dyn Reflect) -> Option<&str> {
    if let Some(string) = key.reflect_as_any().downcast_ref::<String>() {
        return Some(string);
    }
    if let Some(Value::String(string)) = key.reflect_as_any().downcast_ref::<Value>() {
        return Some(string);
    }
    None
}

impl ReflectMap for Value {
    fn reflect_get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        self.as_object()
            .and_then(|map| key_as_str(key).and_then(|k| map.get(k)))
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        match self {
            Value::Object(map) => key_as_str(key).and_then(|k| map.get_mut(k)),
            _ => None,
        }
        .map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set_key(
        &mut self,
        key: &dyn Reflect,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError> {
        let type_name = self.reflect_type_name().to_string();
        let key = key_as_str(key)
            .ok_or_else(|| unsupported("reflect_set_key (non-string key)", &type_name))?
            .to_string();
        let value = into_t::<Value>(value)?;
        match self {
            Value::Object(map) => {
                map.insert(key, value);
                Ok(())
            }
            _ => Err(unsupported("reflect_set_key", &type_name)),
        }
    }

    fn reflect_delete_key(&mut self, key: &dyn Reflect) -> Result<(), ReflectError> {
        let type_name = self.reflect_type_name().to_string();
        let key = key_as_str(key)
            .ok_or_else(|| unsupported("reflect_delete_key (non-string key)", &type_name))?
            .to_string();
        match self {
            Value::Object(map) => match map.remove(&key) {
                Some(_) => Ok(()),
                None => Err(ReflectError::KeyNotFound { key, type_name }),
            },
            _ => Err(unsupported("reflect_delete_key", &type_name)),
        }
    }

    fn reflect_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a dyn Reflect, &'a dyn Reflect)> + 'a> {
        match self.as_object() {
            Some(map) => Box::new(
                map.iter()
                    .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect)),
            ),
            None => Box::new(std::iter::empty()),
        }
    }

    fn reflect_length(&self) -> usize {
        self.as_object().map_or(0, |map| map.len())
    }

    fn reflect_key_type_info(&self) -> &'static TypeInfo {
        type_info::<String>()
    }

    fn reflect_value_type_info(&self) -> &'static TypeInfo {
        type_info::<Value>()
    }
}

impl ReflectSequence for Value {
    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_array()
            .and_then(|array| array.get(index))
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_length(&self) -> usize {
        self.as_array().map_or(0, |array| array.len())
    }

    fn reflect_set_index(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError> {
        let value = into_t::<Value>(value)?;
        match self {
            Value::Array(array) => {
                if index >= array.len() {
                    return Err(ReflectError::IndexOutOfBounds {
                        index,
                        length: array.len(),
                        type_name: type_name::<Self>().to_string(),
                    });
                }
                array[index] = value;
                Ok(())
            }
            _ => Err(unsupported("reflect_set_index", self.reflect_type_name())),
        }
    }

    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_t::<Value>(value)?;
        match self {
            Value::Array(array) => {
                array.push(value);
                Ok(())
            }
            _ => Err(unsupported("reflect_push", self.reflect_type_name())),
        }
    }

    fn reflect_delete_index(&mut self, index: usize) -> Result<(), ReflectError> {
        match self {
            Value::Array(array) => {
                if index >= array.len() {
                    return Err(ReflectError::IndexOutOfBounds {
                        index,
                        length: array.len(),
                        type_name: type_name::<Self>().to_string(),
                    });
                }
                array.remove(index);
                Ok(())
            }
            _ => Err(unsupported(
                "reflect_delete_index",
                self.reflect_type_name(),
            )),
        }
    }

    fn reflect_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Reflect> + 'a> {
        match self.as_array() {
            Some(array) => Box::new(array.iter().map(|value| value as &dyn Reflect)),
            None => Box::new(std::iter::empty()),
        }
    }

    fn reflect_element_type_info(&self) -> &'static TypeInfo {
        type_info::<Value>()
    }
}

impl ReflectEnum for Value {
    fn reflect_active_variant(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::Number(_) => "Number",
            Value::String(_) => "String",
            Value::Array(_) => "Array",
            Value::Object(_) => "Object",
        }
    }

    fn reflect_variants(&self) -> &'static [VariantInfo] {
        static VARIANTS: std::sync::OnceLock<Vec<VariantInfo>> = std::sync::OnceLock::new();
        VARIANTS
            .get_or_init(|| {
                vec![
                    VariantInfo {
                        name: "Null",
                        payload_type_info: None,
                    },
                    VariantInfo {
                        name: "Bool",
                        payload_type_info: Some(type_info::<bool>()),
                    },
                    VariantInfo {
                        name: "Number",
                        payload_type_info: Some(type_info::<Number>()),
                    },
                    VariantInfo {
                        name: "String",
                        payload_type_info: Some(type_info::<String>()),
                    },
                    VariantInfo {
                        name: "Array",
                        payload_type_info: Some(type_info::<Vec<Value>>()),
                    },
                    VariantInfo {
                        name: "Object",
                        payload_type_info: Some(type_info::<Map<String, Value>>()),
                    },
                ]
            })
            .as_slice()
    }

    fn reflect_active_variant_payload(&self) -> Option<&dyn Reflect> {
        match self {
            Value::Null => None,
            Value::Bool(value) => Some(value as &dyn Reflect),
            Value::Number(value) => Some(value as &dyn Reflect),
            Value::String(value) => Some(value as &dyn Reflect),
            Value::Array(value) => Some(value as &dyn Reflect),
            Value::Object(value) => Some(value as &dyn Reflect),
        }
    }

    fn reflect_active_variant_payload_mut(&mut self) -> Option<&mut dyn Reflect> {
        match self {
            Value::Null => None,
            Value::Bool(value) => Some(value as &mut dyn Reflect),
            Value::Number(value) => Some(value as &mut dyn Reflect),
            Value::String(value) => Some(value as &mut dyn Reflect),
            Value::Array(value) => Some(value as &mut dyn Reflect),
            Value::Object(value) => Some(value as &mut dyn Reflect),
        }
    }

    fn reflect_set_variant(
        &mut self,
        variant_name: &str,
        payload: Option<Box<dyn Reflect>>,
    ) -> Result<(), ReflectError> {
        let mismatch = |source: String| ReflectError::ShapeMismatch {
            operation: "reflect_set_variant",
            target: type_name::<Value>().to_string(),
            source,
        };
        match variant_name {
            "Null" => {
                if payload.is_some() {
                    return Err(mismatch("unit variant takes no payload".to_string()));
                }
                *self = Value::Null;
                Ok(())
            }
            "Bool" => {
                *self = Value::Bool(
                    *payload
                        .ok_or_else(|| ReflectError::UnknownVariant {
                            variant: "Bool (missing payload)".to_string(),
                            type_name: type_name::<Value>().to_string(),
                        })?
                        .reflect_into_any()
                        .downcast::<bool>()
                        .map_err(|_| mismatch("payload type mismatch".to_string()))?,
                );
                Ok(())
            }
            "Number" => {
                let payload = payload.ok_or_else(|| ReflectError::UnknownVariant {
                    variant: "Number (missing payload)".to_string(),
                    type_name: type_name::<Value>().to_string(),
                })?;
                *self = Value::Number(
                    *payload
                        .reflect_into_any()
                        .downcast::<Number>()
                        .map_err(|_| mismatch("payload type mismatch".to_string()))?,
                );
                Ok(())
            }
            "String" => {
                let payload = payload.ok_or_else(|| ReflectError::UnknownVariant {
                    variant: "String (missing payload)".to_string(),
                    type_name: type_name::<Value>().to_string(),
                })?;
                *self = Value::String(
                    *payload
                        .reflect_into_any()
                        .downcast::<String>()
                        .map_err(|_| mismatch("payload type mismatch".to_string()))?,
                );
                Ok(())
            }
            "Array" => {
                let payload = payload.ok_or_else(|| ReflectError::UnknownVariant {
                    variant: "Array (missing payload)".to_string(),
                    type_name: type_name::<Value>().to_string(),
                })?;
                *self = Value::Array(
                    *payload
                        .reflect_into_any()
                        .downcast::<Vec<Value>>()
                        .map_err(|_| mismatch("payload type mismatch".to_string()))?,
                );
                Ok(())
            }
            "Object" => {
                let payload = payload.ok_or_else(|| ReflectError::UnknownVariant {
                    variant: "Object (missing payload)".to_string(),
                    type_name: type_name::<Value>().to_string(),
                })?;
                *self = Value::Object(
                    *payload
                        .reflect_into_any()
                        .downcast::<Map<String, Value>>()
                        .map_err(|_| mismatch("payload type mismatch".to_string()))?,
                );
                Ok(())
            }
            other => Err(ReflectError::UnknownVariant {
                variant: other.to_string(),
                type_name: type_name::<Value>().to_string(),
            }),
        }
    }
}

/// `serde_json::Number` is a final scalar value (int/uint/float), not a shape
/// to dive into.
impl Reflect for Number {
    fn reflect_type_name(&self) -> &'static str {
        "number"
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Scalar(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Scalar(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        let source = value
            .reflect_as_any()
            .downcast_ref::<Number>()
            .ok_or_else(|| ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: type_name::<Number>().to_string(),
                source: value.reflect_type_name().to_string(),
            })?;
        *self = source.clone();
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
        ReflectKind::Scalar
    }

    fn reflect_scalar_i64(&self) -> Option<i64> {
        self.as_i64()
            .or_else(|| self.as_u64().and_then(|v| i64::try_from(v).ok()))
    }

    fn reflect_scalar_f64(&self) -> Option<f64> {
        self.as_f64()
    }

    fn reflect_scalar_string(&self) -> Option<String> {
        Some(self.to_string())
    }
}

/// `serde_json::Map<String, Value>` is the object payload type; it routes as
/// a map itself.
impl Reflect for Map<String, Value> {
    fn reflect_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Map(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Map(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        if value.reflect_as_any().type_id() != TypeId::of::<Self>() {
            return Err(ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: type_name::<Self>().to_string(),
                source: value.reflect_type_name().to_string(),
            });
        }
        let source = value
            .reflect_as_any()
            .downcast_ref::<Map<String, Value>>()
            .expect("type id checked");
        *self = source.clone();
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
        ReflectKind::Map
    }
}

impl ReflectMap for Map<String, Value> {
    fn reflect_get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        key_as_str(key)
            .and_then(|k| self.get(k))
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        key_as_str(key)
            .and_then(|k| self.get_mut(k))
            .map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set_key(
        &mut self,
        key: &dyn Reflect,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError> {
        let key = key_as_str(key)
            .ok_or_else(|| unsupported("reflect_set_key (non-string key)", type_name::<Self>()))?
            .to_string();
        self.insert(key, into_t::<Value>(value)?);
        Ok(())
    }

    fn reflect_delete_key(&mut self, key: &dyn Reflect) -> Result<(), ReflectError> {
        let key = key_as_str(key)
            .ok_or_else(|| unsupported("reflect_delete_key (non-string key)", type_name::<Self>()))?
            .to_string();
        match self.remove(&key) {
            Some(_) => Ok(()),
            None => Err(ReflectError::KeyNotFound {
                key,
                type_name: type_name::<Self>().to_string(),
            }),
        }
    }

    fn reflect_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a dyn Reflect, &'a dyn Reflect)> + 'a> {
        Box::new(
            self.iter()
                .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect)),
        )
    }

    fn reflect_length(&self) -> usize {
        self.len()
    }

    fn reflect_key_type_info(&self) -> &'static TypeInfo {
        type_info::<String>()
    }

    fn reflect_value_type_info(&self) -> &'static TypeInfo {
        type_info::<Value>()
    }
}
