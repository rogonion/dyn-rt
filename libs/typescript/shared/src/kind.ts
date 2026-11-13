// Runtime value-kind discrimination (port of the Rust reflect crate's
// ReflectRef routing, anticipated here in `shared`): JavaScript values at
// runtime are more varied than JSON `Value`s — besides the JSON kinds there
// are `undefined`, `bigint`, symbols, functions, arrays, Dates, boxed
// primitives (`new Number(5)`), and user-defined class instances (the
// analogue of Rust structs). `kindOf` tells them apart explicitly; the
// reflect module will build its Reflect* routing on top of this.

import type { JsonObject } from "./value.js";

/**
 * The runtime kind of a value:
 * - `null` / `undefined` — the two "nothing" values (only `null` is a Value)
 * - `boolean` / `number` / `bigint` / `string` / `symbol` — primitives
 * - `function` — callable
 * - `array` — `Array.isArray`
 * - `date` — `instanceof Date`
 * - `plain-object` — prototype is `Object.prototype` or null (JSON object shape)
 * - `class-instance` — any other object: user class instances, boxed
 *   primitives (`new Number(5)` → class-instance with className "Number"),
 *   Map, Set, RegExp, etc.
 */
export type ValueKind =
  | "null"
  | "undefined"
  | "boolean"
  | "number"
  | "bigint"
  | "string"
  | "symbol"
  | "function"
  | "array"
  | "date"
  | "plain-object"
  | "class-instance";

/** Discriminate the runtime kind of any value. */
export function kindOf(value: unknown): ValueKind {
  if (value === null) return "null";
  if (value === undefined) return "undefined";

  const primitiveKind = typeof value;
  if (primitiveKind === "boolean") return "boolean";
  if (primitiveKind === "number") return "number";
  if (primitiveKind === "bigint") return "bigint";
  if (primitiveKind === "string") return "string";
  if (primitiveKind === "symbol") return "symbol";
  if (primitiveKind === "function") return "function";

  // typeof === "object" from here on
  if (Array.isArray(value)) return "array";
  if (value instanceof Date) return "date";
  if (isPlainObject(value)) return "plain-object";
  return "class-instance";
}

/** True for objects whose prototype is `Object.prototype` or null (the JSON object shape). */
export function isPlainObject(value: unknown): value is JsonObject {
  if (typeof value !== "object" || value === null) return false;
  const proto: unknown = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

/** True for non-plain objects: class instances, boxed primitives, Map/Set/RegExp, ... */
export function isClassInstance(value: unknown): boolean {
  return kindOf(value) === "class-instance";
}

/** The constructor name of a class-instance (e.g. "User", "Number"), for error messages. */
export function className(value: object): string {
  const ctor = (value as { constructor?: { name?: string } }).constructor;
  return ctor?.name ?? "Object";
}
