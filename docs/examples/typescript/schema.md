# Examples — `schema` (TypeScript)

Module package: `@dyn-rt/schema` (`libs/typescript/schema`), facade namespace
`schema` from `@dyn-rt/dyn-rt`.

A `Schema` is a `SchemaNode` or a `SchemaSet` (alternatives, go's
`DynamicSchema`). Nodes describe the shape of `unknown` data; leaf `scalar`
nodes name a JS scalar type.

## Scalar: validate + convert

```ts
import { schema } from "@dyn-rt/dyn-rt";

const numSchema = schema.scalar("number");
schema.validate(42, numSchema);                    // ok, returns ValidationOutcome
schema.validate(42, numSchema).validSetKeys;       // []
schema.validate("nope", numSchema);                // throws SchemaError
schema.convert(42, numSchema);                     // 42
```

`scalar` accepts `"number" | "string" | "boolean" | "bigint" | "null"` (plus
JSON scalar variants via `jsonScalar`); `any()` accepts anything.

## Class node over a plain object / class shape

```ts
const userSchema = schema.classNode("User", [
  ["ID", schema.sequenceNode(schema.scalar("number"))],
  ["Name", schema.sequenceNode(schema.scalar("string"))],
]);

schema.validate({ ID: [1], Name: ["Alice"] }, userSchema); // ok
```

Node constructors: `scalar`, `jsonScalar`, `any`, `classNode(typeName,
fields)`, `mapNode(key, value)`, `sequenceNode(elements)`,
`indirectionNode(inner)`, `enumNode(variants)`, plus decorators `nilable`,
`withDefault`, `withValidator`, `withConverter`.

## Schema sets (polymorphic values)

```ts
const set = schema.schemaSet("int", [
  ["int", schema.scalar("number")],
  ["text", schema.scalar("string")],
]);
schema.validate(7, set).validSetKeys; // ["int"] — which alternative matched
```

## Conversion into a target

```ts
// convert(source, schema, target?) — returns the converted value;
// with a target, writes into it.
const out = schema.convert({ ID: [1], Name: ["Alice"] }, userSchema);
```

(Unlike Rust, no serde-style text deserialization step is needed: JSON.parse
already yields the JSON-shaped values the dynamic layer works on.)
