# @dyn-rt/dyn-rt

dyn-rt dynamic runtime — single publishable TypeScript package facading libs/typescript modules via subpath exports (ADR 0003).

The publishable dyn-rt facade: one package re-exporting every module as a namespace (shared, path, reflect, schema, manipulation, metadatamodel) of pure ESM, so bundlers tree-shake what you do not use.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
