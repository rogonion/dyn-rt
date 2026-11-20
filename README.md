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

| Module          | Rust crate                            | npm (via `@dyn-rt/dyn-rt`)        | What it does                                                                                                                                        |
| --------------- | ------------------------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `shared`        | `dyn-rt-shared`                       | `shared`                          | The JSON data model: `Value`, `Number`, `JsonError`, kind helpers                                                                                   |
| `path`          | `dyn-rt-path`                         | `path`                            | JSONPath parsing/resolution over any reflected tree                                                                                                 |
| `reflect`       | `dyn-rt-reflect` (+ `reflect-derive`) | `reflect`                         | General-purpose runtime reflection over native types (Rust rolls its own; TS uses runtime introspection)                                            |
| `schema`        | `dyn-rt-schema`                       | `schema`                          | Schemas over reflected data: validate + convert                                                                                                     |
| `manipulation`  | `dyn-rt-manipulation`                 | `manipulation`                    | `DynObject` entrypoint: get / set / delete / forEach at a JSONPath, deep-equal                                                                      |
| `metadatamodel` | `dyn-rt-metadatamodel`                | `metadatamodel`                   | The metadata model as an extensible JSON-like tree, with subdomains: core, fieldcolumns, database, iter, filter, flattener, unflattener, groupindex |
| `test-support`  | `dyn-rt-test-support` (dev-only)      | `@dyn-rt/test-support` (dev-only) | The data-driven test harness (`TestReport`, `newTestTitle`)                                                                                         |

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

Comprehensive, runnable per-module examples for both ecosystems live in
[**docs/examples/**](docs/examples/README.md) — one doc per module per
language (`docs/examples/rust/<module>.md`,
`docs/examples/typescript/<module>.md`), with sub-sections per feature area
and complex scenarios (nested pointer/class shapes, mixed single/array
cardinality in the metadatamodel).

Quick taste — TypeScript:

```ts
import { manipulation } from "@dyn-rt/dyn-rt";

const user = { ID: [101], Name: ["Alice"], Address: [{ City: ["Nairobi"] }] };
const obj = manipulation.DynObject.new(user);
obj.get("$.Address[*].City[*]"); // => ["Nairobi"]
obj.set("$.Name[0]", "Alicia"); // => 1
```

Rust:

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;

let mut doc = Value::object([("ID", Value::from(vec![Value::from(101i64)]))]);
let mut obj = DynObject::new(&mut doc);
let found = obj.get("$.ID[0]").unwrap(); // 1 value
```

Runnable Rust tours through the published facade crate:

```sh
cargo run -p dyn-rt --example modules --features metadatamodel   # every module
cargo run -p dyn-rt --example schema_deser --features deser      # json/yaml/toml + convert
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
