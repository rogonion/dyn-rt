//! Map adapters: `ReflectMap` for `shared::Value` objects (string keys,
//! insertion order).

use std::any::{Any, TypeId, type_name};

use dyn_rt_shared::Value;

use super::{cloned_t, into_t, into_value, key_as_str, unsupported};
use crate::domain::{
    Reflect, ReflectKind, ReflectMap, ReflectRef, ReflectRefMut, TypeInfo, type_info,
};
use crate::error::ReflectError;

impl ReflectMap for Value {
    fn reflect_get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        self.get(key_as_str(key)?)
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        self.get_mut(key_as_str(key)?)
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
        let value = into_value(value)?;
        match self {
            Value::Object(entries) => {
                match entries.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, slot)) => *slot = value,
                    None => entries.push((key, value)),
                }
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
            Value::Object(entries) => {
                let position = entries.iter().position(|(k, _)| *k == key).ok_or_else(|| {
                    ReflectError::KeyNotFound {
                        key: key.clone(),
                        type_name: type_name.clone(),
                    }
                })?;
                entries.remove(position);
                Ok(())
            }
            _ => Err(unsupported("reflect_delete_key", &type_name)),
        }
    }

    fn reflect_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a dyn Reflect, &'a dyn Reflect)> + 'a> {
        match self.as_object() {
            Some(entries) => Box::new(
                entries
                    .iter()
                    .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect)),
            ),
            None => Box::new(std::iter::empty()),
        }
    }

    fn reflect_length(&self) -> usize {
        self.as_object().map_or(0, |entries| entries.len())
    }

    fn reflect_key_type_info(&self) -> &'static TypeInfo {
        type_info::<String>()
    }

    fn reflect_value_type_info(&self) -> &'static TypeInfo {
        type_info::<Value>()
    }
}

/// `Reflect` + `ReflectMap` for the std map types. A declarative macro is the
/// sanctioned deduplication here (ADR 0005); the two invocations differ only
/// in the key bound (`Hash` vs `Ord`).
macro_rules! impl_reflect_std_map {
    ($map:ty, $($kbound:tt)+) => {
        impl<K: Reflect + $($kbound)+, V: Reflect> Reflect for $map {
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
                Box::new(
                    self.iter()
                        .map(|(k, v)| (cloned_t::<K>(k), cloned_t::<V>(v)))
                        .collect::<$map>(),
                )
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
                    .downcast_ref::<$map>()
                    .expect("type id checked");
                *self = source
                    .iter()
                    .map(|(k, v)| (cloned_t::<K>(k), cloned_t::<V>(v)))
                    .collect();
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

        impl<K: Reflect + $($kbound)+, V: Reflect> ReflectMap for $map {
            fn reflect_get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
                let key = key.reflect_as_any().downcast_ref::<K>()?;
                self.get(key).map(|value| value as &dyn Reflect)
            }

            fn reflect_get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
                let key = key.reflect_as_any().downcast_ref::<K>()?;
                self.get_mut(key).map(|value| value as &mut dyn Reflect)
            }

            fn reflect_set_key(
                &mut self,
                key: &dyn Reflect,
                value: Box<dyn Reflect>,
            ) -> Result<(), ReflectError> {
                let key = into_t::<K>(key.reflect_clone())?;
                let value = into_t::<V>(value)?;
                self.insert(key, value);
                Ok(())
            }

            fn reflect_delete_key(&mut self, key: &dyn Reflect) -> Result<(), ReflectError> {
                let key = key
                    .reflect_as_any()
                    .downcast_ref::<K>()
                    .ok_or_else(|| unsupported("reflect_delete_key (key type mismatch)", self.reflect_type_name()))?;
                match self.remove(key) {
                    Some(_) => Ok(()),
                    None => Err(ReflectError::KeyNotFound {
                        key: key.reflect_type_name().to_string(),
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
                type_info::<K>()
            }

            fn reflect_value_type_info(&self) -> &'static TypeInfo {
                type_info::<V>()
            }
        }
    };
}

impl_reflect_std_map!(std::collections::HashMap<K, V>, Eq + std::hash::Hash);
impl_reflect_std_map!(std::collections::BTreeMap<K, V>, Ord);
