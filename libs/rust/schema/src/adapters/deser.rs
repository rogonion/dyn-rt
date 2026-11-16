//! Deserialization adapters (feature `deser`): parse JSON / YAML / TOML text
//! into `serde_json::Value`, which participates in reflection via
//! dyn-rt-reflect's `serde-json` adapter. This is the only file in the crate
//! that imports the serde format crates (ADR 0002).
//!
//! This is step 1 of 2-step deserialization (see go's `Deserializer`):
//! text -> a reflect-able general value; step 2 is `application::conversion`
//! into the schema-dictated shape. Cross-format parsing into
//! `serde_json::Value` works because it implements `Deserialize` for any
//! self-describing format; XML is intentionally not covered (no serde value
//! enum for it).

use dyn_rt_reflect::Reflect;

use crate::error::SchemaError;

fn map_err(text: &str) -> impl FnOnce(serde_json::Error) -> SchemaError + '_ {
    move |err| SchemaError::DeserializationFailed {
        message: format!("json: {err} in {text:?}"),
    }
}

/// Parses JSON text into a reflect-able value.
pub fn from_json_str(text: &str) -> Result<Box<dyn Reflect>, SchemaError> {
    serde_json::from_str::<serde_json::Value>(text)
        .map(|value| Box::new(value) as Box<dyn Reflect>)
        .map_err(map_err(text))
}

/// Parses YAML text into a reflect-able value (parsed as `serde_json::Value`).
pub fn from_yaml_str(text: &str) -> Result<Box<dyn Reflect>, SchemaError> {
    serde_yaml::from_str::<serde_json::Value>(text)
        .map(|value| Box::new(value) as Box<dyn Reflect>)
        .map_err(|err| SchemaError::DeserializationFailed {
            message: format!("yaml: {err} in {text:?}"),
        })
}

/// Parses TOML text into a reflect-able value (parsed as `serde_json::Value`).
pub fn from_toml_str(text: &str) -> Result<Box<dyn Reflect>, SchemaError> {
    toml::from_str::<serde_json::Value>(text)
        .map(|value| Box::new(value) as Box<dyn Reflect>)
        .map_err(|err| SchemaError::DeserializationFailed {
            message: format!("toml: {err} in {text:?}"),
        })
}
