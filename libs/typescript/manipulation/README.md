# @dyn-rt/manipulation

dyn-rt manipulation module (ADR 0010): get / set / delete / for_each / are_equal over native values at a JSONPath, plus the DynObject entrypoint wrapper. Port of libs/rust/manipulation; TS values are references, so mutation is direct and no reflect-ceremony is needed.

The DynObject entrypoint: get / set / delete / forEach at a JSONPath (string or parsed segments) over any reflected value, schema-guided sets, and deep-equal areEqual.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
