# @dyn-rt/shared

dyn-rt shared module: the JSON value model (`Value` is a type alias over plain JSON values) plus runtime value-kind discrimination for the richer JS runtime (`undefined`, `bigint`, `Date`, class instances, ...).

```ts
import { shared } from "@dyn-rt/dyn-rt";

shared.kindOf({ a: 1 });   // "plain-object"
shared.kindOf([1, 2]);     // "array"
shared.kindOf(new Date()); // "date"
shared.kindOf(new Map());  // "class-instance"
shared.isPlainObject({ a: 1 }); // true
```

Full examples: <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/shared.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer
over a language's own data types: JSONPath-style paths, reflection, schemas,
get/set/delete manipulation, and a metadata-model layer on top. See the repo
README and `docs/architecture/` (ADRs + descriptions, incl. the metadata
model and filter query conditions) for the full picture.

License: MIT (see LICENSE).
