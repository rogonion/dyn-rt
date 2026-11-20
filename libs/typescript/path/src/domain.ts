// JSONPath AST: the deconstructed form of a JSONPath string (port of
// libs/rust/path/src/domain.rs).
//
// A path like `$.store.book[0].title` parses into RecursiveDescentSegments:
// a list of RecursiveDescentSegment groups separated by the recursive
// descent operator `..`, each holding a linear sequence of PathSegments.
//
// The AST is plain data (discriminated unions), so it JSON-compares
// structurally in tests. Rendering (toString rules) follows go's `String()`:
// keys that are plain identifiers join with `.`, everything else
// (bracketed keys, indices, slices, unions) attaches directly, and descent
// groups join with `..`.

/** A JSONPath string, e.g. `$.store.book[0].title`. */
export type JsonPath = string;

/** Hint for how a segment's value is accessed. */
export type AccessStrategy = "Mixed" | "Named" | "Positional";

/** A linear-collection slice `[start:end:step]`; each part is optional. */
export interface SliceSelector {
  start?: number;
  end?: number;
  step?: number;
}

/** The selector of a single path segment. */
export type PathSelector =
  | { kind: "Unrecognized" }
  | { kind: "Root" }
  | { kind: "Wildcard" }
  | { kind: "Key"; key: string }
  | { kind: "Index"; index: number }
  | { kind: "Slice"; slice: SliceSelector }
  | { kind: "Union"; members: PathSegment[] };

/** A single atomic segment of a JSONPath. */
export interface PathSegment {
  accessStrategy: AccessStrategy;
  selector: PathSelector;
}

/** A sequence of path segments directly connected (by `.` or brackets)
 * without any recursive descent operator. */
export type RecursiveDescentSegment = PathSegment[];

/** The deconstructed JSONPath: segment groups split by `..`. */
export type RecursiveDescentSegments = RecursiveDescentSegment[];

// ---------------------------------------------------------------------------
// AST constructor helpers (the TS stand-ins for the Rust
// json_path_*_segment! / new_recursive_descent_segment(s)! macros, so case
// tables read like the paths themselves)
// ---------------------------------------------------------------------------

export function rootSegment(): PathSegment {
  return { accessStrategy: "Mixed", selector: { kind: "Root" } };
}

export function wildcardSegment(): PathSegment {
  return { accessStrategy: "Mixed", selector: { kind: "Wildcard" } };
}

export function keySegment(key: string): PathSegment {
  return { accessStrategy: "Named", selector: { kind: "Key", key } };
}

export function indexSegment(index: number): PathSegment {
  return { accessStrategy: "Positional", selector: { kind: "Index", index } };
}

export function sliceSegment(start?: number, end?: number, step?: number): PathSegment {
  return {
    accessStrategy: "Positional",
    selector: { kind: "Slice", slice: { start, end, step } },
  };
}

export function unionSegment(accessStrategy: AccessStrategy, members: PathSegment[]): PathSegment {
  return { accessStrategy, selector: { kind: "Union", members } };
}

export function descentSegment(...segments: PathSegment[]): RecursiveDescentSegment {
  return segments;
}

export function descentSegments(...groups: RecursiveDescentSegment[]): RecursiveDescentSegments {
  return groups;
}

// ---------------------------------------------------------------------------
// Rendering (Display)
// ---------------------------------------------------------------------------

/** A key renders bare (dot notation) only if it starts with an ASCII letter
 * and contains only ASCII letters, digits, or underscores; otherwise it
 * needs the bracket-quote form `['key']`. */
export function keyNeedsBrackets(key: string): boolean {
  return !/^[A-Za-z][A-Za-z0-9_]*$/.test(key);
}

/** Renders `[start:end:step]`, keeping the colons needed to preserve
 * positions, e.g. `[1:5:2]`, `[1:-5]`, `[:5]`, `[::2]`. */
export function renderSlice(slice: SliceSelector): string {
  let out = "[";
  if (slice.start !== undefined) {
    out += String(slice.start) + ":";
  }
  if (slice.end !== undefined) {
    if (slice.start === undefined) {
      out += ":";
    }
    out += String(slice.end);
  }
  if (slice.step !== undefined) {
    if (slice.start === undefined && slice.end === undefined) {
      out += ":";
    }
    out += ":" + String(slice.step);
  }
  out += "]";
  return out;
}

export function renderSelector(selector: PathSelector): string {
  switch (selector.kind) {
    case "Unrecognized":
      return "";
    case "Root":
      return "$";
    case "Wildcard":
      return "*";
    case "Key":
      return keyNeedsBrackets(selector.key) ? `['${selector.key}']` : selector.key;
    case "Index":
      return `[${selector.index}]`;
    case "Slice":
      return renderSlice(selector.slice);
    case "Union": {
      const parts = selector.members.map((member) => {
        const rendered = renderSelector(member.selector);
        // strip one layer of brackets so members do not double-wrap
        if (rendered.startsWith("[") && rendered.endsWith("]") && rendered.length >= 2) {
          return rendered.slice(1, -1);
        }
        return rendered;
      });
      return `[${parts.join(",")}]`;
    }
  }
}

export function renderSegment(segment: PathSegment): string {
  return renderSelector(segment.selector);
}

/** Joins the rendered segments: the first stands alone; a bracket-form part
 * attaches directly; anything else joins with a `.` separator. */
export function renderDescentSegment(group: RecursiveDescentSegment): string {
  const parts = group.map(renderSegment).filter((rendered) => rendered !== "");
  if (parts.length === 0) {
    return "";
  }

  let result = parts[0]!;
  for (let i = 1; i < parts.length; i++) {
    const part = parts[i]!;
    const partIsBracket = part.startsWith("[") && part.endsWith("]");
    if (partIsBracket) {
      result += part;
    } else {
      if (!result.endsWith(".")) {
        result += ".";
      }
      result += part;
      if (i !== parts.length - 1) {
        const next = parts[i + 1]!;
        const nextIsBracket = next.startsWith("[") && next.endsWith("]");
        if (!nextIsBracket) {
          result += ".";
        }
      }
    }
  }
  return result;
}

/** Joins the non-empty descent groups with `..`. */
export function renderSegments(segments: RecursiveDescentSegments): string {
  const groups = segments.map(renderDescentSegment).filter((rendered) => rendered !== "");
  return groups.join("..");
}

// ---------------------------------------------------------------------------
// PathTokenizer port: roles the core needs from the world. The definition
// lives HERE in domain; implementations live in adapters.
// ---------------------------------------------------------------------------

/** Tokenizer/splitter port used by the parse use cases. Each method is one
 * explicit step of the parse pipeline. */
export interface PathTokenizer {
  /** Step 1: split a path on the recursive descent operator `..`, respecting
   * brackets and quotes (a `..` inside `'...'`/`[...]` is not a delimiter). */
  splitRecursiveDescent(path: string): string[];

  /** Step 2: split a descent part on the dot `.` delimiter, respecting
   * brackets and quotes. */
  splitDotNotation(part: string): string[];

  /** Step 3: split a dot part into bracket contents (the inner text of each
   * `[...]`) and non-bracket text, e.g. `book[0][3]title` -> `"book"`, `"0"`,
   * `"3"`, `"title"`. */
  splitBrackets(part: string): string[];

  /** Step 4a: split a bracket content on union commas `,`, respecting quotes. */
  splitUnion(content: string): string[];

  /** Step 4b: if the token is a quoted string (`'...'`, `"..."`, `` `...` ``),
   * return the unquoted content. */
  extractQuoted(token: string): string | undefined;

  /** Step 4b: if the token is a positional index (`-?digits`), return it. */
  parseIndex(token: string): number | undefined;

  /** Step 4b: if the token is a slice (`start:end:step`, any part optional),
   * return it; `undefined` when the token is not a slice. */
  parseSlice(token: string): SliceSelector | undefined;
}
