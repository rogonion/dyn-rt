# dyn-rt (Rust)

dyn-rt is a **dynamic runtime layer over Rust's own data types** — built-ins
and user structs/enums alike — explored and manipulated at any tree depth via
JSONPath-style paths, reflection, and schemas. A metadata-model layer sits on
top. `json` names only the JSON data model (`Value`/`Number`/`JsonError`), not
the dynamic layer (ADR 0010).

One crate, one cargo feature per module, so dependents only compile what they
use (ADR 0003):

| Feature (module) | What it provides |
|---|---|
| `json` (default) | The JSON value model: `dyn_rt::json::{Value, Number, JsonError}` |
| `path` (default) | JSONPath parsing/resolution over any reflected tree |
| `reflect` (default) | Runtime reflection over native types (`Reflect*` traits, `dyn-rt-reflect` + `reflect-derive`) |
| `schema` (default) | Schemas over reflected data: `validate`, `convert` |
| `manipulation` (default) | `DynObject`: get / set / delete / for_each at a JSONPath; `are_equal` |
| `metadatamodel` | The metadata model as an extensible JSON-like tree + subdomains (core, fieldcolumns, database, iter, filter, flattener, unflattener, groupindex) |
| `deser` | Text deserialization (serde json/yaml/toml) into reflect-able values |

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;

let mut doc = Value::object([
    ("ID", Value::from(vec![Value::from(101i64)])),
    ("Address", Value::from(vec![Value::object([(
        "City",
        Value::from(vec![Value::from("Nairobi")]),
    )])])),
]);

let mut obj = DynObject::new(&mut doc);
let found = obj.get("$.ID[0]").unwrap();        // 1 value
let cities = obj.get("$..City[*]").unwrap();    // recursive descent
obj.set("$.ID[0]", &Value::from(7i64)).unwrap();
obj.delete("$.Address").unwrap();
```

Runnable tour through every module:

```sh
cargo run -p dyn-rt --example modules --features metadatamodel
cargo run -p dyn-rt --example schema_deser --features deser
```

**Full per-module examples (verified, incl. complex nested shapes):**
<https://github.com/rogonion/dyn-rt/tree/main/docs/examples/rust> —
[shared](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/shared.md) ·
[path](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/path.md) ·
[reflect](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/reflect.md) ·
[schema](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/schema.md) ·
[manipulation](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/manipulation.md) ·
[metadatamodel](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/metadatamodel.md)

See the repository for full docs: <https://github.com/rogonion/dyn-rt>
(ADRs in `docs/architecture/ADRs/`, long-form descriptions in
`docs/architecture/descriptions/` — e.g. the metadata model and filter query
conditions).

License: MIT (see LICENSE).
