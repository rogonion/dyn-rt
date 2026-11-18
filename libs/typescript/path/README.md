# @dyn-rt/path

dyn-rt path module (ADR 0010): JSONPath AST + parse use cases over any reflected tree. Port of libs/rust/path. Hexagon inside: domain (std only), application, adapters (regex tokenizer).

JSONPath parsing and resolution over any reflected tree: keys, indexes, slices, unions (incl. negative indexes), wildcards, recursive descent. Filter selectors are not supported.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
