# Examples — `shared` (TypeScript)

Module package: `@dyn-rt/shared` (`libs/typescript/shared`), facade namespace
`shared` from `@dyn-rt/dyn-rt`.

In TypeScript the JSON data model **is** plain JavaScript values (`Value` is a
type alias over JSON values), so `shared` mainly adds runtime **kind
discrimination** — JS values are more varied than JSON: `undefined`,
`bigint`, `symbol`, functions, `Date`, boxed primitives, and user class
instances.

## kindOf — tell the runtime kinds apart

```ts
import { shared } from "@dyn-rt/dyn-rt";

shared.kindOf({ a: 1 });        // "plain-object"  (JSON object shape)
shared.kindOf([1, 2]);          // "array"
shared.kindOf("x");             // "string"
shared.kindOf(10n);             // "bigint"
shared.kindOf(undefined);       // "undefined"
shared.kindOf(new Date());      // "date"
shared.kindOf(new Map());       // "class-instance"
shared.kindOf(new (class Foo {})()); // "class-instance" (user classes — the struct analogue)

shared.isPlainObject({ a: 1 }); // true
shared.typeName({ a: 1 });      // "object"
```

`ValueKind`: `"null" | "undefined" | "boolean" | "number" | "bigint" |
"string" | "symbol" | "function" | "array" | "date" | "plain-object" |
"class-instance"`.

Everything downstream (path, manipulation, schema, metadatamodel) accepts
`unknown` and routes on these kinds — plain objects and arrays are the JSON
shape; class instances are traversed by field name.
