# @dyn-rt/reflect

dyn-rt reflect module (ADR 0010): runtime reflection over native JS/TS values — kind routing + sequence/map/struct/scalar accessors, deep clone, zero, apply. Port of libs/rust/reflect; TS needs no trait machinery, shared's kindOf does the routing.

Runtime reflection over native TS values mapped onto the dyn-rt shape vocabulary (Scalar / Sequence / Map / Class / Unknown), usable independently of the rest of dyn-rt.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
