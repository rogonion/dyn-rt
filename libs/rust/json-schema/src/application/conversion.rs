//! Conversion use cases: convert reflected data into the shape a schema
//! dictates (port of go-json's conversion.go) — stage 2, not yet implemented.
//!
//! The entry point will be `convert(data: &dyn Reflect, schema: &Schema,
//! target: &mut dyn Reflect)`, walking the schema and driving the target
//! through the reflect `ReflectMut` shape traits (set field by name, push
//! elements, set inner of indirections, set variant of enums).
