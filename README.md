# dyn-rt

dyn-rt is a **dynamic runtime layer over a language's own data types** —
built-in types and user-defined structs/classes alike — explored and
manipulated at any tree depth via JSONPath-style paths, reflection, and
schemas. A metadata-model layer sits on top. `json` names only the JSON data
model (the `Value` type and JSON text), not the dynamic layer (ADR 0010).

Guided port of the reference implementations in `../go-json`,
`../go-metadatamodel`, `../js-json`, `../js-metadatamodel` — Rust first
(`libs/rust`, `packages/rust`), then a TypeScript mirror (`libs/typescript`,
`packages/typescript`).

Each ecosystem ships ONE package with per-module gating so consumers only
bundle what they use (ADR 0003): the `dyn-rt` crate (cargo features) and the
`@dyn-rt/dyn-rt` npm package (namespace re-exports of pure ESM modules).
See `docs/architecture/ADRs/` for the decisions and
`docs/architecture/descriptions/` for long-form docs (e.g.
[the metadata model and filter query conditions](docs/architecture/descriptions/metadatamodel.md)).

## Modules

| Module | Rust crate | npm (via `@dyn-rt/dyn-rt`) | What it does |
|---|---|---|---|
| `shared` | `dyn-rt-shared` | `shared` | The JSON data model: `Value`, `Number`, `JsonError`, kind helpers |
| `path` | `dyn-rt-path` | `path` | JSONPath parsing/resolution over any reflected tree |
| `reflect` | `dyn-rt-reflect` (+ `reflect-derive`) | `reflect` | General-purpose runtime reflection over native types (Rust rolls its own; TS uses runtime introspection) |
| `schema` | `dyn-rt-schema` | `schema` | Schemas over reflected data: validate + convert |
| `manipulation` | `dyn-rt-manipulation` | `manipulation` | `DynObject` entrypoint: get / set / delete / forEach at a JSONPath, deep-equal |
| `metadatamodel` | `dyn-rt-metadatamodel` | `metadatamodel` | The metadata model as an extensible JSON-like tree, with subdomains: core, fieldcolumns, database, iter, filter, flattener, unflattener, groupindex |
| `test-support` | `dyn-rt-test-support` (dev-only) | `@dyn-rt/test-support` (dev-only) | The data-driven test harness (`TestReport`, `newTestTitle`) |

## Layout

```
libs/       one crate/package per module (ADR 0007): shared, test-support (dev-only),
            path, reflect, schema, manipulation, metadatamodel —
            each with src/{domain, application, adapters} inside
            (metadatamodel: one subdir per subdomain, each domain/application/adapters)
packages/   publishable export entrypoints (one per ecosystem, ADR 0003)
docs/       architecture ADRs + descriptions
misc/       scratch notes
```

This is a library repo — there is no `apps/`; executables/composition roots
live in consuming projects.

## Examples

Per-module snippets in both ecosystems. The Rust snippets below are a
condensed form of the runnable tour in
[packages/rust/dyn-rt/examples/modules.rs](packages/rust/dyn-rt/examples/modules.rs):

```sh
cargo run -p dyn-rt --example modules --features metadatamodel
```

### Rust

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;
use dyn_rt::reflect::{ReflectStruct, kind_of};
use dyn_rt::schema::{Schema, SchemaNode, convert, validate};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: Vec<i64>,
    #[reflect(rename = "Name")]
    name: Vec<String>,
}

// json — the JSON data model (Value / Number / JsonError)
let mut doc = Value::object([
    ("ID", Value::from(vec![Value::from(101i64)])),
    ("Name", Value::from(vec![Value::from("Alice")])),
]);

// path — parse a JSONPath into segments (Display renders it back)
let segments = dyn_rt::path::parse("$.ID[0]"); // "$.ID[0]"

// reflect — runtime reflection over the native struct
let user = User { id: vec![1], name: vec!["Alice".to_string()] };
kind_of(&user); // ReflectKind::Struct

// schema — validate + convert
let schema = Schema::Node(SchemaNode::scalar::<i64>());
validate(&42i64, &schema).unwrap();
let mut target = 0i64;
convert(&42i64, &schema, &mut target).unwrap(); // target == 42

// manipulation — DynObject get / set / delete at a JSONPath
let mut obj = DynObject::new(&mut doc);
obj.get("$.ID[0]").unwrap();                      // 1 value
obj.set("$.Name[0]", &Value::from("Alicia")).unwrap();
obj.delete("$.ID").unwrap();

// metadatamodel — post-process a group:path_key template, flatten to a 2D table
let data_path = dyn_rt::metadatamodel::core::data_path(
    "$['group:fields'][*].ID[*]", &[], dyn_rt::metadatamodel::core::Cardinality::Array,
).unwrap(); // "$.ID[0]"
let mut flattener = dyn_rt::metadatamodel::flattener::Flattener::new(&model).unwrap();
flattener.flatten(&DynObject::read_only(&user)).unwrap();
flattener.result();
```

### TypeScript

```ts
import { shared, path, schema, manipulation, metadatamodel } from "@dyn-rt/dyn-rt";

// any native structure works — plain objects, arrays, class instances, ...
const user = { ID: [101], Name: ["Alice"], Address: [{ City: ["Nairobi"] }] };

// reflect — kindOf distinguishes built-ins and class instances
shared.kindOf(user); // "plain-object"

// path — parse a JSONPath into segments
path.parsePath(new path.RegexPathTokenizer(), "$.ID[0]");

// schema — validate + convert
const numSchema = schema.scalar("number");
schema.validate(42, numSchema);
schema.convert(42, numSchema); // 42

// manipulation — explore/set/delete any structure at any depth with a JSONPath
const obj = manipulation.DynObject.new(user);
obj.get("$.Address[*].City[*]");  // => ["Nairobi"]
obj.set("$.Name[0]", "Alicia");   // => 1
obj.delete("$.ID");               // => 1

// metadatamodel — a metadata model is a plain value: fields/groups with
// <section>:<key> props, always arrays down to the field props
const model = {
  [metadatamodel.core.props.GROUP_PATH_KEY]: "$",
  [metadatamodel.core.FIELDS_KEY]: [
    {
      ID: [{ [metadatamodel.core.props.GROUP_PATH_KEY]: "$['group:fields'][*].ID[*]",
             [metadatamodel.core.props.GROUP_IS_PRIMARY_KEY]: true }],
    },
  ],
};
metadatamodel.core.dataPath("$['group:fields'][*].ID[*]", [], "array"); // "$.ID[0]"

// flatten nested data to a 2D table, filter rows by query conditions, ...
const flattener = new metadatamodel.flattener.Flattener(model);
flattener.flatten(manipulation.DynObject.new(user));
flattener.result();                                   // => [[[101]]]
const dataFilter = new metadatamodel.filter.DataFilter(
  manipulation.DynObject.new(rows), model,
);
dataFilter.filter(queryCondition); // indexes that did NOT pass the filter
```

See [docs/architecture/descriptions/metadatamodel.md](docs/architecture/descriptions/metadatamodel.md)
for a comprehensive nested metadata model (groups, `db:*` column mapping,
cardinality, separate columns) and the full JSON structure of filter
query conditions.

## Commands

Run everything from the repo root via uv + poe:

```sh
uv run poe fmt     # cargo fmt
uv run poe lint    # cargo clippy + pnpm lint
uv run poe test    # cargo test --workspace + pnpm -r test
uv run poe build   # cargo build --workspace + pnpm -r build
```

Or directly: `cargo test --workspace` for Rust, `pnpm -r test` for TypeScript.
