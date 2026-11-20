# @dyn-rt/schema

dyn-rt schema module (ADR 0010): JSON-like schema trees over native values — validation, conversion, schema-path resolution. Port of libs/rust/schema; TS values are plain data, so no reflect ceremony is needed.

Schemas over reflected data (Scalar, Any, Class, Map, Sequence, Indirection, Enum, plus sets): validate a value, convert it into the schema shape.

```ts
import { schema } from "@dyn-rt/dyn-rt";

const numSchema = schema.scalar("number");
schema.validate(42, numSchema);              // ok; .validSetKeys => []
schema.convert(42, numSchema);               // 42

const userSchema = schema.classNode("User", [
  ["ID", schema.sequenceNode(schema.scalar("number"))],
  ["Name", schema.sequenceNode(schema.scalar("string"))],
]);
schema.validate({ ID: [1], Name: ["Alice"] }, userSchema);

const set = schema.schemaSet("int", [
  ["int", schema.scalar("number")],
  ["text", schema.scalar("string")],
]);
schema.validate(7, set).validSetKeys;        // ["int"]
```

Full examples: <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/schema.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
