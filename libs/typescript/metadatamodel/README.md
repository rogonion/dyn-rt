# @dyn-rt/metadatamodel

dyn-rt metadatamodel (TS): the metadata model as an extensible JSON-like Value tree — field/group props, templated group:path_key paths, strict cardinality, plus the fieldcolumns / database / iter / filter / flattener / unflattener / groupindex subdomains. Port of libs/rust/metadatamodel.

The metadata model as an extensible JSON-like tree (section:key props, templated group:path_key, strict single/array cardinality) with subdomains: core, fieldcolumns, database, iter, filter, flattener, unflattener, groupindex.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
