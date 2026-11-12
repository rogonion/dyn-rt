//! Map adapters: `ReflectMap` for `shared::Value` objects (string keys,
//! insertion order).

use dyn_rt_shared::Value;

use super::{into_value, key_as_str, unsupported};
use crate::domain::{Reflect, ReflectMap, TypeInfo, type_info};
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
