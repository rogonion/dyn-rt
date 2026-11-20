# Examples

Runnable, verified examples for every module, in both ecosystems. Each doc
mirrors the `libs/` layout and is organized in sub-sections per feature
area; every snippet shown was executed against the current code (complex
nested shapes with pointers/class instances included, and mixed
single/array cardinality for the metadatamodel).

| Module                                                                                           | Rust                                           | TypeScript                                                 |
| ------------------------------------------------------------------------------------------------ | ---------------------------------------------- | ---------------------------------------------------------- |
| `shared` (JSON value model; facade `dyn_rt::json` in Rust)                                       | [rust/shared.md](rust/shared.md)               | [typescript/shared.md](typescript/shared.md)               |
| `path` (JSONPath)                                                                                | [rust/path.md](rust/path.md)                   | [typescript/path.md](typescript/path.md)                   |
| `reflect` (runtime reflection)                                                                   | [rust/reflect.md](rust/reflect.md)             | [typescript/reflect.md](typescript/reflect.md)             |
| `schema` (validate / convert)                                                                    | [rust/schema.md](rust/schema.md)               | [typescript/schema.md](typescript/schema.md)               |
| `manipulation` (`DynObject` get/set/delete/forEach)                                              | [rust/manipulation.md](rust/manipulation.md)   | [typescript/manipulation.md](typescript/manipulation.md)   |
| `metadatamodel` (core, fieldcolumns, database, iter, filter, flattener, unflattener, groupindex) | [rust/metadatamodel.md](rust/metadatamodel.md) | [typescript/metadatamodel.md](typescript/metadatamodel.md) |

Runnable code:

```sh
# Rust — per-module tour through the published facade crate
cargo run -p dyn-rt --example modules --features metadatamodel
# Rust — 2-step text deserialization (json/yaml/toml) + schema conversion
cargo run -p dyn-rt --example schema_deser --features deser
```

TypeScript snippets import from the published facade:

```ts
import { shared, path, reflect, schema, manipulation, metadatamodel } from "@dyn-rt/dyn-rt";
```

Long-form design docs live in `docs/architecture/` (ADRs +
descriptions, incl. the full metadata-model JSON and filter query
conditions).
