// Group-index domain (port of
// libs/rust/metadatamodel/src/groupindex/domain.rs, itself extracted from
// go-metadatamodel/unflattener's index.go): a node of instances, the
// collection of a group's instances keyed by signature, and the signature
// generator.

import type { Value } from "@dyn-rt/shared";

/** A specific instance of a group element (e.g. "Employee #1") (go's
 * `GroupIndexNode`). */
export class GroupIndexNode {
  /** Nested group collections, keyed by group suffix. */
  groups: [string, GroupCollection][] = [];

  constructor(
    public pathKey: string,
    /** The resolved index of this node in its parent's list. */
    public myIndex: number,
  ) {}

  /** Retrieves or creates the child collection for a nested group suffix. */
  getOrCreateGroup(suffix: string): GroupCollection {
    const existing = this.groups.find(([s]) => s === suffix);
    if (existing !== undefined) {
      return existing[1];
    }
    const collection = new GroupCollection();
    this.groups.push([suffix, collection]);
    return collection;
  }
}

/** A "list" of child nodes of one group (go's `GroupCollection`). */
export class GroupCollection {
  /** Counter for the next available index in this list. */
  nextIndex = 0;
  /** Instances keyed by signature (insertion-ordered). */
  instances: [string, GroupIndexNode][] = [];

  /** Retrieves or creates the instance for `signature`, returning its index
   * in this list and its node (go's `GetOrCreateInstance`). */
  getOrCreateInstance(signature: string, pathKey: string): [number, GroupIndexNode] {
    const existing = this.instances.find(([s]) => s === signature);
    if (existing !== undefined) {
      return [existing[1].myIndex, existing[1]];
    }
    const index = this.nextIndex;
    this.nextIndex += 1;
    const node = new GroupIndexNode(pathKey, index);
    this.instances.push([signature, node]);
    return [index, node];
  }
}

/** Generates unique row keys from primary-key columns (go's `Signature`). */
export class Signature {
  constructor(public joinSymbol = "|") {}

  /** A deterministic key for `row` over the given column indexes. */
  generateSignature(row: Value[], columns: number[]): string {
    if (columns.length === 0) {
      return "";
    }
    const parts: string[] = [];
    for (const col of columns) {
      const value = row[col];
      if (value === undefined) {
        parts.push("nil");
        continue;
      }
      parts.push(renderSignatureValue(value));
    }
    return parts.join(this.joinSymbol);
  }
}

/** Renders one cell value for a signature: strings as-is, numbers/bools
 * compactly, null/empty as `nil`. */
function renderSignatureValue(value: Value): string {
  if (value === null) return "nil";
  if (typeof value === "boolean") return value ? "t" : "f";
  if (typeof value === "number") return String(value);
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(renderSignatureValue).join(",");
  return JSON.stringify(value);
}
