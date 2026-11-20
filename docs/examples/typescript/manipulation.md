# Examples — `manipulation` (TypeScript)

Module package: `@dyn-rt/manipulation` (`libs/typescript/manipulation`),
facade namespace `manipulation` from `@dyn-rt/dyn-rt`.

`DynObject` is the entrypoint: wrap a source with `DynObject.new(source)`,
then `get` / `set` / `delete` / `forEach` at a JSONPath (string or parsed
AST). TS has no ownership, so there is a single `new` (no `readOnly`/`owned`).

## get — wildcards, negative indexes, unions, recursive descent

```ts
import { manipulation } from "@dyn-rt/dyn-rt";

const doc = {
  ID: [101, 102],
  Address: [{ City: ["Nairobi", "Mombasa"] }],
};
const obj = manipulation.DynObject.new(doc);

obj.get("$.ID[*]");      // 2 values
obj.get("$.ID[-1]");     // 1 value (negative index)
obj.get("$.ID[0,1]");    // 2 values (union)
obj.get("$..City[*]");   // ["Nairobi", "Mombasa"] (recursive descent)
```

`get` returns every match: `unknown[]`.

## set / delete / forEach

```ts
obj.set("$.ID[0]", 7);   // => 1 (how many values were set)
obj.delete("$.ID[1]");   // => 1
// doc.ID is now [7]

let count = 0;
obj.forEach("$..City[*]", () => { count += 1; return true; }); // false terminates
// count === 2
```

Attach a schema to guide coercion/creation on `set`:
`DynObject.new(doc).withSchema(schema)` (or per-call `setWithSchema`).

## Deep equality

```ts
manipulation.areEqual({ a: [1] }, { a: [1] }); // true
manipulation.DynObject.areEqual(left, right);  // static form
```

## Complex nested shapes

Class instances, plain objects, arrays and tuples-as-arrays nest freely:

```ts
class Address {
  City: string[] = [];
}
class Complex {
  Meta: Record<string, [number, string][]> = {};
  Maybe?: Address;
}

const c = new Complex();
c.Meta = { k1: [[1, "one"], [2, "two"]] };
c.Maybe = new Address();
c.Maybe.City = ["Nairobi"];

const cobj = manipulation.DynObject.new(c);
cobj.get("$.Meta['k1'][0][1]");   // ["one"]  (object key -> array index -> tuple index)
cobj.get("$.Maybe.City[0]");      // ["Nairobi"] (through the optional class field)
cobj.set("$.Maybe.City[0]", "Mombasa"); // => 1
// c.Maybe.City is now ["Mombasa"]
```
