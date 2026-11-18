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
use dyn_rt::{json::Value, manipulation::DynObject, path};

let mut doc = Value::object([("name", Value::from("dyn-rt"))]);
let obj = DynObject::new(&mut doc);
let found = obj.get(&path::parse_path(...)); // ...
```

See the repository for full docs: <https://github.com/rogonion/dyn-rt>
(ADRs in `docs/architecture/ADRs/`, long-form descriptions in
`docs/architecture/descriptions/` — e.g. the metadata model and filter query
conditions).

License: MIT (see LICENSE).
