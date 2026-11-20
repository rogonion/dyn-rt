# @dyn-rt/reflect

dyn-rt reflect module: runtime introspection helpers routed on `shared.kindOf` — sequences (arrays), maps (plain objects), class instances (the struct analogue), scalars, deep clone / zero. Port of libs/rust/reflect.

```ts
import { reflect, shared } from "@dyn-rt/dyn-rt";

class User {
  ID: number[] = [];
  Name: string[] = [];
  meta: Record<string, [number, string][]> = {};
}
const user = new User();
user.ID = [1];
user.meta = { k1: [[1, "one"]] };

shared.kindOf(user); // "class-instance"
reflect.asSequence(user.ID); // [1]
reflect.asMap(user.meta); // { k1: [[1, "one"]] }
reflect.classFieldNames(user); // ["ID", "Name", "meta"]
reflect.classGet(user, "ID"); // [1]
reflect.scalarString("x"); // "x"
reflect.reflectClone(user); // deep clone
```

Full examples (incl. complex nested class shapes): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/reflect.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
