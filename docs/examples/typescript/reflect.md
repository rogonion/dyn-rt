# Examples — `reflect` (TypeScript)

Module package: `@dyn-rt/reflect` (`libs/typescript/reflect`), facade
namespace `reflect` from `@dyn-rt/dyn-rt`.

TS has runtime introspection but no Rust-style trait objects, so `reflect`
is a set of explicit accessor functions routed on `shared.kindOf`. It is
usable on its own, independent of the other modules.

## Sequence accessors (arrays)

```ts
import { reflect } from "@dyn-rt/dyn-rt";

reflect.asSequence([1, 2, 3]); // [1, 2, 3] (undefined for non-arrays)
reflect.seqLength([1, 2, 3]); // 3
reflect.seqGet([1, 2, 3], 1); // 2
reflect.seqSet([1, 2, 3], 1, 9); // in place
reflect.seqPush([1], 2); // in place
reflect.seqDeleteIndex([1, 2], 0); // in place
```

## Map accessors (plain objects, string keys)

```ts
reflect.asMap({ a: 1 }); // { a: 1 } (undefined for class instances)
reflect.mapGet({ a: 1 }, "a"); // 1
reflect.mapSet(obj, "b", 2); // in place
reflect.mapDelete(obj, "a"); // true (false when absent)
reflect.mapContainsKey(obj, "a"); // boolean
reflect.mapKeys(obj); // string[]
reflect.mapLength(obj); // number
```

## Class accessors (user classes — the struct analogue)

```ts
class Address {
  City: string[] = [];
}
class User {
  ID: number[] = [];
  Name: string[] = [];
  address?: Address;
  meta: Record<string, [number, string][]> = {};
}

const user = new User();
user.ID = [1];
user.Name = ["Alice"];
user.address = new Address();
user.address.City = ["Nairobi"];
user.meta = {
  k1: [
    [1, "one"],
    [2, "two"],
  ],
};

reflect.classFieldNames(user); // ["ID", "Name", "address", "meta"]
reflect.classGet(user, "Name"); // ["Alice"]
reflect.classSet(user, "Name", ["Alicia"]); // in place
reflect.asClass(user); // the instance (undefined for plain objects)
```

## Scalars, clone, zero

```ts
reflect.scalarNumber(42); // 42 (undefined for non-numbers)
reflect.scalarBigint(10n); // 10n
reflect.scalarBool(true); // true
reflect.scalarString("x"); // "x"

const cloned = reflect.reflectClone(user); // deep clone
reflect.reflectZero(42); // 0 — zero value of the same kind
```

## Complex nested shapes

`manipulation.DynObject` traverses class instances, plain objects, arrays,
and tuples-as-arrays at any depth — see
[manipulation.md](manipulation.md#complex-nested-shapes) for a full example
(`$.Meta['k1'][0][1]`, `$.Maybe.City[0]`, ...).
