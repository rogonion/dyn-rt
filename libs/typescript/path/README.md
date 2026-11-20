# @dyn-rt/path

dyn-rt path module (ADR 0010): JSONPath AST + parse use cases over any reflected tree. Port of libs/rust/path. Hexagon inside: domain (std only), application, adapters (regex tokenizer).

JSONPath parsing and resolution over any reflected tree: keys, indexes, slices, unions (incl. negative indexes), wildcards, recursive descent. Filter selectors are not supported.

```ts
import { path } from "@dyn-rt/dyn-rt";

const tokenizer = new path.RegexPathTokenizer();
const segments = path.parsePath(tokenizer, "$.store.book[0].title");
path.renderSegments(segments);                              // "$.store.book[0].title"
path.renderSegments(path.parsePath(tokenizer, "$[0,-1]"));  // "$[0,-1]"  union, negative index
path.renderSegments(path.parsePath(tokenizer, "$.a[1:8:2]")); // "$.a[1:8:2]" slice
path.renderSegments(path.parsePath(tokenizer, "$..City[*]")); // "$..City.*" recursive descent
```

Full examples: <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/path.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
