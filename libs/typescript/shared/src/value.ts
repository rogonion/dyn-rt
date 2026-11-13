// The pure JSON value model (port of libs/rust/shared/src/value.rs).
//
// Unlike the Rust `Value` enum, the TypeScript `Value` is a plain type alias
// over native JSON values: objects are JS objects (string keys, insertion
// order preserved), arrays are JS arrays, and numbers are JS `number`
// (IEEE-754 double — big integers beyond 2^53 go through `bigint`, which is
// NOT a `Value`; see kind.ts for distinguishing them at runtime).

/** A JSON object: string keys to Values, insertion-ordered. */
export type JsonObject = { [key: string]: Value };

/** A JSON value. */
export type Value = null | boolean | number | string | Value[] | JsonObject;

/** Human-readable JSON type name, used in JsonError.TypeMismatch. */
export function typeName(value: Value): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return "boolean";
  if (typeof value === "number") return "number";
  if (typeof value === "string") return "string";
  if (Array.isArray(value)) return "array";
  return "object";
}

export function isNull(value: Value): value is null {
  return value === null;
}

export function isBoolean(value: Value): value is boolean {
  return typeof value === "boolean";
}

export function isNumber(value: Value): value is number {
  return typeof value === "number";
}

export function isString(value: Value): value is string {
  return typeof value === "string";
}

export function isArray(value: Value): value is Value[] {
  return Array.isArray(value);
}

export function isObject(value: Value): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function asArray(value: Value): Value[] | undefined {
  return isArray(value) ? value : undefined;
}

export function asObject(value: Value): JsonObject | undefined {
  return isObject(value) ? value : undefined;
}

/** Object member lookup by key (undefined when not an object or key absent). */
export function get(value: Value, key: string): Value | undefined {
  const obj = asObject(value);
  if (obj === undefined) return undefined;
  return obj[key];
}

/**
 * Build an object from key/value pairs; a duplicate key replaces the earlier
 * entry (last wins), matching the Rust `Value::object` dedupe.
 */
export function object(entries: Iterable<readonly [string, Value]>): JsonObject {
  const result: JsonObject = {};
  for (const [key, val] of entries) {
    result[key] = val;
  }
  return result;
}
