// Validation use cases (port of libs/rust/schema/src/application/validation.rs):
// check data against a schema, walking plain JS values via the reflect
// accessors. Throws SchemaError on the first failure.

import { kindOf } from "@dyn-rt/shared";
import {
  isSchemaSet,
  renderPath,
  type Schema,
  type SchemaNode,
  type SchemaPath,
} from "../domain.js";
import { SchemaError } from "../error.js";

/** Options for validation (port of go's `Validation` class). */
export interface ValidationOptions {
  /** Stop at the first matching node of a set (go's default: true). */
  validateOnFirstMatch?: boolean;
  /** Custom validators keyed by the data's runtime kind. */
  customValidators?: Record<string, (data: unknown, node: SchemaNode, path: SchemaPath) => void>;
}

/** The outcome of a successful validation: which set keys matched. */
export interface ValidationOutcome {
  validSetKeys: string[];
}

export function isNil(value: unknown): boolean {
  return value === null || value === undefined;
}

/** Validates `data` against `schema` with default options. */
export function validate(data: unknown, schema: Schema, options: ValidationOptions = {}): ValidationOutcome {
  const outcome: ValidationOutcome = { validSetKeys: [] };
  validateSchema(data, schema, [], options, outcome);
  return outcome;
}

/** Validates `data` against a single schema node with default options. */
export function validateNode(data: unknown, node: SchemaNode, path: SchemaPath = []): void {
  validateNodeWith(data, node, path, {}, { validSetKeys: [] });
}

function fail(path: SchemaPath, message: string): SchemaError {
  return SchemaError.validationFailed(message, renderPath(path));
}

function validateSchema(
  data: unknown,
  schema: Schema,
  path: SchemaPath,
  options: ValidationOptions,
  outcome: ValidationOutcome,
): void {
  if (isSchemaSet(schema)) {
    validateSet(data, schema, path, options, outcome);
  } else {
    validateNodeWith(data, schema, path, options, outcome);
  }
}

function validateSet(
  data: unknown,
  set: { defaultKey?: string; nodes: [string, SchemaNode][] },
  path: SchemaPath,
  options: ValidationOptions,
  outcome: ValidationOutcome,
): void {
  if (set.nodes.length === 0) {
    throw fail(path, "no schema nodes found");
  }
  const firstMatch = options.validateOnFirstMatch ?? true;
  let lastErr: unknown;
  const ordered = [...set.nodes].sort(([a], [b]) =>
    a === set.defaultKey ? -1 : b === set.defaultKey ? 1 : 0,
  );
  for (const [key, node] of ordered) {
    try {
      validateNodeWith(data, node, path, options, outcome);
      outcome.validSetKeys.push(key);
      if (firstMatch) return;
    } catch (err) {
      lastErr = err;
    }
  }
  if (outcome.validSetKeys.length === 0) {
    throw lastErr instanceof Error ? lastErr : fail(path, "data does not match any schema node");
  }
}

function validateNodeWith(
  data: unknown,
  node: SchemaNode,
  path: SchemaPath,
  options: ValidationOptions,
  outcome: ValidationOutcome,
): void {
  if (isNil(data)) {
    if (node.nilable) return;
    throw fail(path, "data cannot be nil");
  }

  const custom = options.customValidators?.[kindOf(data)];
  if (custom !== undefined) {
    custom(data, node, path);
    return;
  }
  if (node.validator !== undefined) {
    node.validator.validate(data, node, path);
    return;
  }

  switch (node.shape.kind) {
    case "Any":
      return;
    case "Scalar": {
      const expected = scalarKind(node.typeName);
      if (expected !== undefined && kindOf(data) !== expected) {
        throw fail(path, `expected ${expected}, found ${kindOf(data)}`);
      }
      return;
    }
    case "Class": {
      if (kindOf(data) !== "plain-object" && kindOf(data) !== "class-instance") {
        throw fail(path, `expected class, found ${kindOf(data)}`);
      }
      const record = data as Record<string, unknown>;
      for (const [fieldName, fieldSchema] of node.shape.fields) {
        const childPath = [...path, { kind: "Key", key: fieldName } as const];
        const present = Object.prototype.hasOwnProperty.call(record, fieldName);
        if (!present) {
          if (node.childrenMustBeValid) {
            throw fail(childPath, `missing required field '${fieldName}'`);
          }
          continue;
        }
        validateSchema(record[fieldName], fieldSchema, childPath, options, outcome);
      }
      return;
    }
    case "Map": {
      // maps are plain objects; a class instance is a class, not a map
      if (kindOf(data) !== "plain-object") {
        throw fail(path, `expected map, found ${kindOf(data)}`);
      }
      const record = data as Record<string, unknown>;
      const shape = node.shape;
      if (shape.entries.length === 0 && (shape.key === undefined || shape.value === undefined)) {
        throw fail(path, "no schema to validate entries in data (map) found");
      }
      for (const key of Object.keys(record)) {
        const childPath = [...path, { kind: "Key", key } as const];
        const entry = shape.entries.find(([k]) => k === key);
        const valueSchema = entry !== undefined ? entry[1] : shape.value;
        if (valueSchema === undefined) {
          throw fail(childPath, `schema for map key '${key}' not found`);
        }
        if (shape.key !== undefined) {
          validateSchema(coerceMapKey(key, shape.key), shape.key, childPath, options, outcome);
        }
        validateSchema(record[key], valueSchema, childPath, options, outcome);
      }
      return;
    }
    case "Sequence": {
      if (!Array.isArray(data)) {
        throw fail(path, `expected array, found ${kindOf(data)}`);
      }
      const shape = node.shape;
      for (let index = 0; index < data.length; index++) {
        const childPath = [...path, { kind: "Index", index } as const];
        const item = shape.items.find(([i]) => i === index);
        const elementSchema = item !== undefined ? item[1] : shape.elements;
        if (elementSchema === undefined) {
          throw fail(childPath, "no schema for sequence element found");
        }
        validateSchema(data[index], elementSchema, childPath, options, outcome);
      }
      return;
    }
    case "Indirection":
      validateSchema(data, node.shape.inner, path, options, outcome);
      return;
    case "Enum": {
      // an enum-shaped value is { variant, payload? }; a bare (non-object)
      // value validates against the "Some" variant's payload — the
      // Option-like case, e.g. a plain string against Option<string>
      if (kindOf(data) === "plain-object" || kindOf(data) === "class-instance") {
        const record = data as Record<string, unknown>;
        const active = record.variant;
        if (typeof active !== "string" || !node.shape.variants.some(([name]) => name === active)) {
          throw fail(path, `data is not a valid enum variant of '${node.typeName}'`);
        }
        const [, variantSchema] = node.shape.variants.find(([name]) => name === active)!;
        if (variantSchema !== undefined) {
          validateSchema(record.payload, variantSchema, path, options, outcome);
        }
        return;
      }
      const some = node.shape.variants.find(([name]) => name === "Some");
      if (some !== undefined && some[1] !== undefined) {
        validateSchema(data, some[1], path, options, outcome);
        return;
      }
      throw fail(path, `data is not a valid enum variant of '${node.typeName}'`);
    }
  }
}

/** JS object keys are always strings; a scalar key schema applies to the
 * key's coerced form (e.g. "1" against an int key schema validates as 1). */
function coerceMapKey(key: string, keySchema: Schema): unknown {
  if (!isSchemaSet(keySchema) && keySchema.shape.kind === "Scalar") {
    switch (keySchema.typeName) {
      case "number":
      case "float":
      case "f64":
      case "int":
      case "i64": {
        const parsed = Number(key);
        return Number.isNaN(parsed) || key.trim() === "" ? key : parsed;
      }
      case "bigint": {
        try {
          return BigInt(key);
        } catch {
          return key;
        }
      }
      case "boolean":
      case "bool":
        if (key === "true") return true;
        if (key === "false") return false;
        return key;
      default:
        return key;
    }
  }
  return key;
}

/** The runtime kind a scalar type name expects ("number" -> "number", ...). */
function scalarKind(typeName: string): string | undefined {
  switch (typeName) {
    case "number":
    case "float":
    case "f64":
    case "int":
    case "i64":
      return "number";
    case "bigint":
      return "bigint";
    case "boolean":
    case "bool":
      return "boolean";
    case "string":
      return "string";
    case "null":
      return "null";
    default:
      return undefined; // unknown scalar label: accept any scalar
  }
}
