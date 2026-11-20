# Examples — `path` (Rust)

Module crate: `dyn-rt-path` (`libs/rust/path`), facade `dyn_rt::path`.
Feature `path` (in `default`).

Parses JSONPath strings into a deconstructed AST (`RecursiveDescentSegments`)
via named regexes (`RegexPathTokenizer` adapter), and every AST node has a
`Display` impl that renders it back to path text.

## Parsing

```rust
use dyn_rt::path::{self, RegexPathTokenizer, parse_path};

let segments = path::parse("$.store.book[0].title");
println!("{segments}"); // $.store.book[0].title

// explicit tokenizer form (same result):
let same = parse_path(&RegexPathTokenizer, "$.store.book[0].title");
```

## Selector kinds

```rust
use dyn_rt::path;

println!("{}", path::parse("$[0,-1]"));    // union incl. negative index: $[0,-1]
println!("{}", path::parse("$.a[1:8:2]"));  // slice: $.a[1:8:2]
println!("{}", path::parse("$.a[*]"));      // wildcard: $.a.*
println!("{}", path::parse("$..City[*]"));  // recursive descent: $..City.*
```

The AST: `RecursiveDescentSegments(Vec<RecursiveDescentSegment>)`, each
segment holding a `PathSelector` — `Root`, `Key(String)`, `Index(isize)`
(negative indexes supported), `Slice{..}`, `Union(Vec<PathSelector>)`,
`Wildcard`, `RecursiveDescent`, or `Unrecognized`.

## Unsupported: filter selectors

Filter selectors (`[?(@.price < 10)]`) are intentionally not supported; the
token parses as an unrecognized bracket key instead of failing:

```rust
use dyn_rt::path;

let segs = path::parse("$[?(@.price < 10)]");
println!("{segs}"); // $['?(@.price < 10)']
```

## Single-segment extraction

```rust
use dyn_rt::path::{RegexPathTokenizer, extract_segment};

let seg = extract_segment(&RegexPathTokenizer, "[0]");
println!("{seg}"); // [0]
```

## Using a parsed path with manipulation

`DynObject::get`/`set`/`delete` accept a path string, a parsed
`RecursiveDescentSegments`, or a single segment — see
[manipulation.md](manipulation.md).
