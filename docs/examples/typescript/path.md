# Examples — `path` (TypeScript)

Module package: `@dyn-rt/path` (`libs/typescript/path`), facade namespace
`path` from `@dyn-rt/dyn-rt`.

Parses JSONPath strings into a deconstructed AST via named regexes
(`RegexPathTokenizer` adapter); `renderSegments` renders an AST back to path
text (go's `String()` rules).

## Parsing + rendering

```ts
import { path } from "@dyn-rt/dyn-rt";

const tokenizer = new path.RegexPathTokenizer();
const segments = path.parsePath(tokenizer, "$.store.book[0].title");
path.renderSegments(segments); // "$.store.book[0].title"
```

## Selector kinds

```ts
path.renderSegments(path.parsePath(tokenizer, "$[0,-1]"));   // "$[0,-1]"  union, negative index
path.renderSegments(path.parsePath(tokenizer, "$.a[1:8:2]")); // "$.a[1:8:2]" slice
path.renderSegments(path.parsePath(tokenizer, "$.a[*]"));     // "$.a.*"    wildcard
path.renderSegments(path.parsePath(tokenizer, "$..City[*]")); // "$..City.*" recursive descent
```

AST: `RecursiveDescentSegments` → `RecursiveDescentSegment[]` → `PathSegment`
with a `PathSelector`: `Root`, `Key`, `Index` (negative supported), `Slice`,
`Union`, `Wildcard`, `RecursiveDescent`, or `Unrecognized`.

## Unsupported: filter selectors

```ts
// filter selectors are intentionally not supported; the token parses as an
// unrecognized bracket key instead of failing:
path.renderSegments(path.parsePath(tokenizer, "$[?(@.price < 10)]"));
// "$['?(@.price < 10)']"
```

## Using a parsed path with manipulation

`DynObject.get/set/delete/forEach` accept a path string or a parsed AST —
see [manipulation.md](manipulation.md).
