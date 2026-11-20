# dyn-rt-path

JSONPath parsing into a deconstructed AST (`RecursiveDescentSegments`) via named regexes (`RegexPathTokenizer` adapter), with `Display` rendering back to path text. Keys, indexes (incl. negative), slices, unions, wildcards, recursive descent. Filter selectors intentionally unsupported. Usable independently; facade: `dyn_rt::path`.

```rust
use dyn_rt::path;

let segments = path::parse("$.store.book[0].title");
assert_eq!(segments.to_string(), "$.store.book[0].title");
assert_eq!(path::parse("$[0,-1]").to_string(), "$[0,-1]");   // union, negative index
assert_eq!(path::parse("$.a[1:8:2]").to_string(), "$.a[1:8:2]"); // slice
assert_eq!(path::parse("$..City[*]").to_string(), "$..City.*"); // recursive descent
```

Full examples: <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/path.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
