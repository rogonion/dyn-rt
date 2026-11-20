# @dyn-rt/dyn-rt

dyn-rt dynamic runtime — single publishable TypeScript package facading libs/typescript modules via subpath exports (ADR 0003).

The publishable dyn-rt facade: one package re-exporting every module as a namespace (shared, path, reflect, schema, manipulation, metadatamodel) of pure ESM, so bundlers tree-shake what you do not use.

```ts
import { manipulation, metadatamodel } from "@dyn-rt/dyn-rt";

const user = { ID: [101], Name: ["Alice"], Address: [{ City: ["Nairobi"] }] };
const obj = manipulation.DynObject.new(user);
obj.get("$.Address[*].City[*]"); // => ["Nairobi"]
obj.set("$.Name[0]", "Alicia");  // => 1

metadatamodel.core.dataPath("$['group:fields'][*].ID[*]", [], "array"); // "$.ID[0]"
```

**Full per-module examples (verified, incl. complex nested class shapes and
mixed single/array cardinality):**
<https://github.com/rogonion/dyn-rt/tree/main/docs/examples/typescript> —
[shared](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/shared.md) ·
[path](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/path.md) ·
[reflect](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/reflect.md) ·
[schema](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/schema.md) ·
[manipulation](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/manipulation.md) ·
[metadatamodel](https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/metadatamodel.md)

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
