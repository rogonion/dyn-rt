// `areEqual`: recursive structural equality over values (port of
// libs/rust/manipulation/src/application/are_equal.rs, itself ported from
// go-json's object.AreEqual).
//
// Sequences, maps (plain objects) and classes (class instances) are compared
// element/entry/field-wise (maps by key, so entry order does not matter);
// scalars by kind plus value; enum-shaped values by variant plus payload;
// Some payloads are unwrapped before comparing (like go unwrapping Ptr at
// every level). Custom per-constructor checkers can be registered (go's
// AreEquals map keyed by reflect.Type).

import { isPlainObject, kindOf } from "@dyn-rt/shared";
import { isEnumValue, unwrapAll } from "./helpers.js";

/** A custom equality checker for one concrete class (keyed by constructor in
 * the map passed to areEqual). */
export type CustomEqualFn = (left: unknown, right: unknown) => boolean;

export type CustomEqualMap = Map<new (...args: never[]) => unknown, CustomEqualFn>;

/** Recursively checks whether `left` and `right` are structurally equal. */
export function areEqual(left: unknown, right: unknown, custom?: CustomEqualMap): boolean {
  // unwrap Some payloads on both sides (go unwraps Ptr/Interface at every level)
  const l = unwrapAll(left);
  const r = unwrapAll(right);

  if (custom !== undefined && l !== null && typeof l === "object") {
    const checker = custom.get(l.constructor as new (...args: never[]) => unknown);
    if (checker !== undefined) return checker(l, r);
  }

  const lKind = kindOf(l);
  if (lKind !== kindOf(r)) return false;

  switch (lKind) {
    case "null":
    case "undefined":
      return true;
    case "boolean":
    case "number":
    case "bigint":
    case "string":
      return l === r;
    case "array": {
      const lSeq = l as unknown[];
      const rSeq = r as unknown[];
      if (lSeq.length !== rSeq.length) return false;
      for (let i = 0; i < lSeq.length; i++) {
        if (!areEqual(lSeq[i], rSeq[i], custom)) return false;
      }
      return true;
    }
    case "plain-object": {
      // map equality is key-matched, so insertion order does not matter
      const lMap = l as Record<string, unknown>;
      const rMap = r as Record<string, unknown>;
      const lKeys = Object.keys(lMap);
      const rKeys = Object.keys(rMap);
      if (lKeys.length !== rKeys.length) return false;
      for (const key of lKeys) {
        if (!Object.prototype.hasOwnProperty.call(rMap, key)) return false;
        if (!areEqual(lMap[key], rMap[key], custom)) return false;
      }
      return true;
    }
    case "class-instance": {
      // enum-shaped values compare by variant + payload
      if (isEnumValue(l) || isEnumValue(r)) {
        if (!isEnumValue(l) || !isEnumValue(r)) return false;
        if (l.variant !== r.variant) return false;
        return areEqual(l.payload, r.payload, custom);
      }
      // classes compare by field name sets + field values (not by constructor)
      const lRec = l as Record<string, unknown>;
      const rRec = r as Record<string, unknown>;
      const lKeys = Object.keys(lRec);
      const rKeys = Object.keys(rRec);
      if (lKeys.length !== rKeys.length) return false;
      for (const key of lKeys) {
        if (!Object.prototype.hasOwnProperty.call(rRec, key)) return false;
        if (!areEqual(lRec[key], rRec[key], custom)) return false;
      }
      return true;
    }
    case "date":
      return (l as Date).getTime() === (r as Date).getTime();
    default:
      // symbol / function: only reference equality
      return l === r;
  }
}

/** True when both values are plain objects (map comparison applies). */
export function isMapLike(value: unknown): value is Record<string, unknown> {
  return isPlainObject(value);
}
