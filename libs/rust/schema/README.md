# dyn-rt-schema

Schemas over reflected data: `Schema` = `SchemaNode` | `SchemaSet`. `validate` checks a `&dyn Reflect` against a schema (outcome reports which set keys matched); `convert` converts a source into a target as the schema dictates. Feature `deser` adds text deserialization (serde json/yaml/toml) into reflect-able values. Facade: `dyn_rt::schema`.

```rust
use dyn_rt::schema::{Schema, SchemaNode, convert, validate};

let schema = Schema::Node(SchemaNode::scalar::<i64>());
validate(&42i64, &schema).unwrap();
assert!(validate(&String::from("nope"), &schema).is_err());

let mut target = 0i64;
convert(&42i64, &schema, &mut target).unwrap();
assert_eq!(target, 42);
```

Full examples (struct/enum/map/sequence nodes, sets, deser + convert): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/schema.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
