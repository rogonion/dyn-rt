// Reflect domain (port of libs/rust/reflect/src/domain.rs, adapted to TS):
// the ReflectRef routing over native values. TypeScript has runtime
// introspection, so the Rust trait system collapses to a kind routing
// function built on shared's kindOf, plus the shape names the manipulation
// layer switches on.

import { className, kindOf } from "@dyn-rt/shared";

/** The shape of a value, mirroring Rust's `ReflectRef` variants:
 * - `Scalar` — primitives and null/undefined (leaves)
 * - `Sequence` — arrays
 * - `Map` — plain objects (string keys)
 * - `Class` — user class instances (fixed-ish field set, prototype kept)
 * - `Unknown` — opaque leaves we do not traverse into (Date, Map, Set,
 *   RegExp, boxed primitives, functions); `Enum`/`Indirection` have no TS
 *   runtime analogue (TS enums are plain objects; there are no
 *   pointer wrappers) and route to `Unknown`. */
export type ReflectRef =
  | "Scalar"
  | "Sequence"
  | "Map"
  | "Class"
  | "Enum"
  | "Indirection"
  | "Unknown";

/** Route a value to its reflect shape. */
export function reflectRef(value: unknown): ReflectRef {
  switch (kindOf(value)) {
    case "array":
      return "Sequence";
    case "plain-object":
      return "Map";
    case "class-instance":
      // boxed primitives (new Number/String/Boolean) are scalar leaves
      if (
        value instanceof Number ||
        value instanceof String ||
        value instanceof Boolean
      ) {
        return "Scalar";
      }
      return "Class";
    case "null":
    case "undefined":
    case "boolean":
    case "number":
    case "bigint":
    case "string":
    case "symbol":
      return "Scalar";
    default:
      // date, function, and any other object kind: opaque leaf
      return "Unknown";
  }
}

/** A human-readable type name for error messages (kind, or class name for
 * class instances). */
export function reflectTypeName(value: unknown): string {
  const kind = kindOf(value);
  if (kind === "class-instance") {
    return className(value as object);
  }
  return kind;
}
