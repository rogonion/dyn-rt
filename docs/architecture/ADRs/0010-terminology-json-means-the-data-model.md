# ADR 0010 — Terminology: `json` names the JSON data model, not the dynamic layer

- Status: accepted
- Date: 2026-11-11
- Amends: ADR 0007 (module names)

## Context

The modules were named `json-path`, `json-manipulation`, `json-schema`, which
suggests the library is about JSON documents. That misreads the intention:
dyn-rt is a **dynamic runtime layer over a language's own data types** —
built-in types and user-defined structs/classes alike, explored and
manipulated at any tree depth via JSONPath-style paths, reflection, and
schemas. JSON is just the most common data shape it interoperates with (the
name stems from the dynamic web UI origins, where JS objects serialize to
JSON).

## Decision

- `json` refers ONLY to the JSON data model and JSON text:
  - `shared::Value` / `Number` / `JsonError` — the JSON-shaped value model
    (re-exported as `dyn_rt::json::{Value, Number, JsonError}` in the façade),
  - the text (de)serialization adapters (serde json/yaml/toml; feature `deser`).
- The dynamic-layer modules drop the `json-` prefix:
  - `json-path` → `path` (crate `dyn-rt-path`, npm `@dyn-rt/path`)
  - `json-manipulation` → `manipulation` (`dyn-rt-manipulation`, `@dyn-rt/manipulation`)
  - `json-schema` → `schema` (`dyn-rt-schema`, `@dyn-rt/schema`)
  - `reflect`, `shared`, `test-support`, `metadatamodel` keep their names.
- "JSONPath" remains the name of the path language itself (the `path` module
  parses and resolves JSONPaths over any reflected tree).
- Façade (`packages/*/dyn-rt`): features `path`, `schema`, `manipulation`,
  `reflect`, `metadatamodel`, `deser`; modules `dyn_rt::path`,
  `dyn_rt::schema`, `dyn_rt::manipulation`, `dyn_rt::json` (value model only),
  `dyn_rt::reflect`, `dyn_rt::metadatamodel`.
- The TypeScript mirror uses the new names from the start
  (`libs/typescript/path`, ...), so no rename is needed there.

## Consequences

- Crate/dir renames are mechanical; older ADRs (0003, 0004, 0007) keep the
  historical `json-*` names and are superseded by this ADR where they conflict.
- `DynObject` (not `Object`) is the manipulation entrypoint name in both
  languages because `Object` collides with TypeScript's global.
