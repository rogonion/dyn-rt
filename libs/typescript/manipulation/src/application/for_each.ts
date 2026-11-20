// `forEach`: invoke a callback for every value found at a JSONPath in a source
// value (port of libs/rust/manipulation/src/application/for_each.rs, itself
// ported from go-json's object.ForEach).
//
// The traversal is the same one `get` uses, so every selector form get
// supports (recursive descent, wildcard, unions incl. negative/slice members,
// slices) applies here too. The callback returns `false` to stop the
// iteration early (go's `IfValueFoundInObject` bool).

import type { RecursiveDescentSegments } from "@dyn-rt/path";
import { get } from "./get.js";

/** Calls `callback` for each value in `source` matched by `segments`, in
 * traversal order. Returning `false` from the callback stops early. */
export function forEach(source: unknown, segments: RecursiveDescentSegments, callback: (value: unknown) => boolean): void {
  for (const value of get(source, segments)) {
    if (!callback(value)) {
      break;
    }
  }
}
