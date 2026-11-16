// Schema domain model (port of libs/rust/schema/src/domain.rs): a JSON-like
// schema tree describing the expected structure of data. In TS the concrete
// `TypeInfo` collapses to a `typeName` display label; shapes are a
// discriminated union so each variant carries exactly the child schemas it
// uses.

// ---------------------------------------------------------------------------
// schema paths
// ---------------------------------------------------------------------------

export type SchemaPathSegment =
  | { kind: "Root" }
  | { kind: "Key"; key: string }
  | { kind: "Index"; index: number };

export type SchemaPath = SchemaPathSegment[];

/** Renders a schema path in JSONPath form, e.g. `$.Address.Street`. */
export function renderPath(path: SchemaPath): string {
  let out = "";
  for (const segment of path) {
    if (segment.kind === "Root") {
      out += "$";
    } else if (segment.kind === "Key") {
      if (/^[A-Za-z][A-Za-z0-9_]*$/.test(segment.key)) {
        out += "." + segment.key;
      } else {
        out += `['${segment.key}']`;
      }
    } else {
      out += `[${segment.index}]`;
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// custom hooks
// ---------------------------------------------------------------------------

/** Custom per-node validation logic (port of go's `Validator`). */
export interface SchemaValidator {
  validate(data: unknown, node: SchemaNode, path: SchemaPath): void; // throws SchemaError
}

/** Custom per-node conversion logic (port of go's `Converter`). */
export interface SchemaConverter {
  convert(data: unknown, node: SchemaNode, path: SchemaPath): unknown; // throws SchemaError
}

// ---------------------------------------------------------------------------
// schema model
// ---------------------------------------------------------------------------

/** A schema: either a single node or a set of alternative nodes. */
export type Schema = SchemaNode | SchemaSet;

/** A collection of alternative schema nodes (go's `DynamicSchema`), e.g. for
 * polymorphic values. `defaultKey` is tried first during validation,
 * conversion, and path resolution. */
export interface SchemaSet {
  defaultKey?: string;
  nodes: [string, SchemaNode][];
}

export function isSchemaSet(schema: Schema): schema is SchemaSet {
  return "nodes" in schema;
}

/** A single node in the schema tree (go's `DynamicSchemaNode`). */
export interface SchemaNode {
  /** Display label of the expected type ("number", "User", ...). */
  typeName: string;
  shape: SchemaShape;
  /** Whether the data may be nil (null/undefined). */
  nilable?: boolean;
  /** Optional default value constructor for new initializations. */
  default?: () => unknown;
  /** When true, every child schema entry must be matched by the data. */
  childrenMustBeValid?: boolean;
  /** Custom validator for this specific node. */
  validator?: SchemaValidator;
  /** Custom converter for this specific node. */
  converter?: SchemaConverter;
}

export type SchemaShape =
  | { kind: "Scalar" }
  | { kind: "Any" }
  | { kind: "Struct"; fields: [string, Schema][] }
  | { kind: "Map"; entries: [string, Schema][]; key?: Schema; value?: Schema }
  | { kind: "Sequence"; elements?: Schema; items: [number, Schema][] }
  | { kind: "Indirection"; inner: Schema }
  | { kind: "Enum"; variants: [string, Schema | undefined][] };

// ---------------------------------------------------------------------------
// builders
// ---------------------------------------------------------------------------

function base(typeName: string, shape: SchemaShape): SchemaNode {
  return { typeName, shape };
}

/** A scalar node for the named scalar type ("number", "string", "boolean",
 * "bigint", "null", or a json scalar variant). */
export function scalar(typeName: string): SchemaNode {
  return base(typeName, { kind: "Scalar" });
}

/** A scalar node for a JSON scalar variant ("string", "number", "boolean",
 * "null") — produces plain JS values. */
export function jsonScalar(variantName: string): SchemaNode {
  return base(variantName, { kind: "Scalar" });
}

/** A node accepting any value. */
export function any(): SchemaNode {
  return base("any", { kind: "Any" });
}

export function structNode(typeName: string, fields: [string, Schema][]): SchemaNode {
  return base(typeName, { kind: "Struct", fields });
}

export function mapNode(key: Schema, value: Schema): SchemaNode {
  return base("map", { kind: "Map", entries: [], key, value });
}

export function mapNodeWithEntries(
  entries: [string, Schema][],
  key?: Schema,
  value?: Schema,
): SchemaNode {
  return base("map", { kind: "Map", entries, key, value });
}

export function sequenceNode(elements: Schema, items: [number, Schema][] = []): SchemaNode {
  return base("array", { kind: "Sequence", elements, items });
}

export function indirectionNode(inner: Schema): SchemaNode {
  return base("indirection", { kind: "Indirection", inner });
}

export function enumNode(variants: [string, Schema | undefined][]): SchemaNode {
  return base("enum", { kind: "Enum", variants });
}

export function schemaSet(defaultKey: string | undefined, nodes: [string, SchemaNode][]): SchemaSet {
  return { defaultKey, nodes };
}

// ---------------------------------------------------------------------------
// node modifiers (chainable, mirroring the Rust builder methods)
// ---------------------------------------------------------------------------

export function nilable(node: SchemaNode): SchemaNode {
  return { ...node, nilable: true };
}

export function childrenMustBeValid(node: SchemaNode): SchemaNode {
  return { ...node, childrenMustBeValid: true };
}

export function withDefault(node: SchemaNode, defaultFn: () => unknown): SchemaNode {
  return { ...node, default: defaultFn };
}

export function withValidator(node: SchemaNode, validator: SchemaValidator): SchemaNode {
  return { ...node, validator };
}

export function withConverter(node: SchemaNode, converter: SchemaConverter): SchemaNode {
  return { ...node, converter };
}
