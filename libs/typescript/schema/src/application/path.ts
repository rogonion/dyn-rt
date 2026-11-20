// Schema-path resolution (port of libs/rust/schema/src/application/path.rs):
// look up the schema node for a linear JSONPath segment within a larger
// schema. Paths must be linear (no recursive descent `..`).

import type { PathSegment, RecursiveDescentSegment } from "@dyn-rt/path";
import { isSchemaSet, type Schema, type SchemaNode } from "../domain.js";
import { SchemaError } from "../error.js";

/** The result of resolving a path against a schema: the node at that path
 * plus the set keys that matched along the way. */
export interface SchemaAtPath {
  node: SchemaNode;
  validSetKeys: string[];
}

/** Resolves a single recursive-descent segment (a linear path) against `schema`. */
export function schemaAtPath(segment: RecursiveDescentSegment, schema: Schema): SchemaAtPath {
  const validSetKeys: string[] = [];
  const node = resolveAt(0, segment, schema, validSetKeys);
  return { node, validSetKeys };
}

function resolveAt(index: number, segments: PathSegment[], schema: Schema, validSetKeys: string[]): SchemaNode {
  if (segments.length === 0 || index >= segments.length) {
    throw SchemaError.schemaPathError("current path segment indexes exhausted");
  }
  if (isSchemaSet(schema)) {
    return resolveSetAt(index, segments, schema, validSetKeys);
  }
  return resolveNodeAt(index, segments, schema, validSetKeys);
}

function resolveSetAt(index: number, segments: PathSegment[], set: { defaultKey?: string; nodes: [string, SchemaNode][] }, validSetKeys: string[]): SchemaNode {
  if (set.nodes.length === 0) {
    throw SchemaError.schemaPathError("no schema nodes found");
  }
  let lastErr: unknown;
  const tryNode = (key: string, node: SchemaNode): SchemaNode => {
    try {
      const resolved = resolveNodeAt(index, segments, node, validSetKeys);
      validSetKeys.push(key);
      return resolved;
    } catch (err) {
      lastErr = err;
      throw err;
    }
  };
  const ordered = [...set.nodes].sort(([a], [b]) => (a === set.defaultKey ? -1 : b === set.defaultKey ? 1 : 0));
  for (const [key, node] of ordered) {
    try {
      return tryNode(key, node);
    } catch {
      // try the next node
    }
  }
  throw lastErr instanceof Error ? lastErr : SchemaError.schemaPathError("no schema nodes found");
}

function resolveNodeAt(index: number, segments: PathSegment[], node: SchemaNode, validSetKeys: string[]): SchemaNode {
  const last = segments.length - 1;
  const segment = segments[index]!;
  const selector = segment.selector;

  if (selector.kind === "Root") {
    if (index === last) return node;
    return resolveNodeAt(index + 1, segments, node, validSetKeys);
  }
  if (selector.kind === "Key" || selector.kind === "Index") {
    const collectionKey = selector.kind === "Key" ? selector.key : String(selector.index);
    return resolveWithCollectionKey(index, segments, node, collectionKey, validSetKeys);
  }
  throw SchemaError.schemaPathError("current path segment is not key or index");
}

function resolveWithCollectionKey(index: number, segments: PathSegment[], node: SchemaNode, collectionKey: string, validSetKeys: string[]): SchemaNode {
  const last = segments.length - 1;
  const shape = node.shape;

  // a pointer does not consume a path segment: resolve into the inner schema
  if (shape.kind === "Indirection") {
    return resolveAt(index, segments, shape.inner, validSetKeys);
  }
  if (shape.kind === "Map") {
    const entry = shape.entries.find(([k]) => k === collectionKey);
    if (entry !== undefined) {
      return index === last ? schemaAsNode(entry[1], validSetKeys) : resolveAt(index + 1, segments, entry[1], validSetKeys);
    }
    if (shape.key === undefined || shape.value === undefined) {
      throw SchemaError.schemaPathError("schema for associative collection keys and/or values not found");
    }
    if (index === last) return schemaAsNode(shape.value, validSetKeys);
    return resolveAt(index + 1, segments, shape.value, validSetKeys);
  }
  if (shape.kind === "Sequence") {
    const item = shape.items.find(([i]) => String(i) === collectionKey);
    if (item !== undefined) {
      return index === last ? schemaAsNode(item[1], validSetKeys) : resolveAt(index + 1, segments, item[1], validSetKeys);
    }
    if (shape.elements === undefined) {
      throw SchemaError.schemaPathError("schema for linear collection elements not found");
    }
    return index === last ? schemaAsNode(shape.elements, validSetKeys) : resolveAt(index + 1, segments, shape.elements, validSetKeys);
  }
  if (shape.kind === "Class") {
    if (shape.fields.length === 0) {
      throw SchemaError.schemaPathError("schema child nodes empty");
    }
    const field = shape.fields.find(([name]) => name === collectionKey);
    if (field === undefined) {
      throw SchemaError.schemaPathError(`schema for class field '${collectionKey}' not found`);
    }
    return index === last ? schemaAsNode(field[1], validSetKeys) : resolveAt(index + 1, segments, field[1], validSetKeys);
  }
  // leaf shapes: no children to descend into; return the node itself
  return node;
}

/** Reduces a schema to a single node: a node passes through, a set yields its
 * default node (or the first node when no default key is set). */
function schemaAsNode(schema: Schema, validSetKeys: string[]): SchemaNode {
  if (!isSchemaSet(schema)) return schema;
  const set = schema;
  if (set.defaultKey !== undefined) {
    const hit = set.nodes.find(([key]) => key === set.defaultKey);
    if (hit !== undefined) {
      validSetKeys.push(set.defaultKey);
      return hit[1];
    }
  }
  const first = set.nodes[0];
  if (first === undefined) {
    throw SchemaError.schemaPathError("no schema nodes found");
  }
  validSetKeys.push(first[0]);
  return first[1];
}
