# @dyn-rt/manipulation

dyn-rt manipulation module: `DynObject` — the entrypoint wrapping any source value — with `get` / `set` / `delete` / `forEach` at a JSONPath (string or parsed AST), plus `areEqual` deep comparison. Port of libs/rust/manipulation.

```ts
import { manipulation } from "@dyn-rt/dyn-rt";

const doc = {
  ID: [101, 102],
  Address: [{ City: ["Nairobi", "Mombasa"] }],
};
const obj = manipulation.DynObject.new(doc);

obj.get("$.ID[*]"); // 2 values
obj.get("$.ID[-1]"); // negative index
obj.get("$..City[*]"); // ["Nairobi", "Mombasa"] — recursive descent
obj.set("$.ID[0]", 7); // => 1
obj.delete("$.ID[1]"); // => 1

manipulation.areEqual({ a: [1] }, { a: [1] }); // true
```

Works on plain objects, arrays, and class instances at any depth, e.g.
`obj.get("$.Maybe.City[0]")` through an optional nested class field.

Full examples (incl. complex nested class shapes): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/manipulation.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
