// Conversion use cases (port of libs/rust/schema/src/application/conversion.rs):
// convert data into the shape a schema dictates. TS is dynamically typed, so
// instead of driving a `&mut dyn Reflect` target in place, `convert` builds
// and returns the converted value; when a `target` is passed and both are
// struct-shaped, the result keeps the target's prototype. Scalar coercions
// follow go's convertTo{Int,Float,String,Bool}.

import { reflectClone } from "@dyn-rt/reflect";
import { isClassInstance, isPlainObject } from "@dyn-rt/shared";
import {
  isSchemaSet,
  renderPath,
  type Schema,
  type SchemaNode,
  type SchemaPath,
} from "../domain.js";
import { SchemaError } from "../error.js";

/** Options for conversion (port of go's `Conversion` struct). */
export interface ConversionOptions {
  /** Custom converters keyed by the SOURCE data's type name. */
  customConverters?: Record<string, (data: unknown, node: SchemaNode, path: SchemaPath) => unknown>;
}

/** The outcome of a successful conversion: which set keys matched. */
export interface ConversionOutcome {
  validSetKeys: string[];
}

export interface ConversionResult extends ConversionOutcome {
  value: unknown;
}

function isNil(value: unknown): boolean {
  return value === null || value === undefined;
}

/** Converts `source` against `schema` into a new value (target optional, for
 * prototype preservation on struct shapes). Throws SchemaError. */
export function convert(source: unknown, schema: Schema, target?: unknown): unknown {
  return convertWith(source, schema, target, {}).value;
}

/** Converts with explicit options; returns the value plus the matched set keys. */
export function convertWith(
  source: unknown,
  schema: Schema,
  target?: unknown,
  options: ConversionOptions = {},
): ConversionResult {
  const outcome: ConversionOutcome = { validSetKeys: [] };
  const value = convertSchema(source, schema, target, [], options, outcome);
  return { value, validSetKeys: outcome.validSetKeys };
}

function convertSchema(
  source: unknown,
  schema: Schema,
  target: unknown,
  path: SchemaPath,
  options: ConversionOptions,
  outcome: ConversionOutcome,
): unknown {
  if (isSchemaSet(schema)) {
    return convertSet(source, schema, target, path, options, outcome);
  }
  return convertNode(source, schema, target, path, options, outcome);
}

/** Converts `source` against a single schema node. */
export function convertNode(
  source: unknown,
  node: SchemaNode,
  target?: unknown,
  path: SchemaPath = [],
  options: ConversionOptions = {},
  outcome: ConversionOutcome = { validSetKeys: [] },
): unknown {
  return convertNodeWith(source, node, target, path, options, outcome);
}

function fail(path: SchemaPath, message: string): SchemaError {
  return SchemaError.conversionFailed(message, renderPath(path));
}

function convertNodeWith(
  source: unknown,
  node: SchemaNode,
  target: unknown,
  path: SchemaPath,
  options: ConversionOptions,
  outcome: ConversionOutcome,
): unknown {
  if (isNil(source)) {
    if (node.nilable) {
      return target === undefined ? null : target; // leave the target as-is
    }
    throw fail(path, "source cannot be nil");
  }

  if (node.shape.kind === "Any") {
    return reflectClone(source);
  }

  // custom converters run before anything else: an explicit converter wins
  if (node.converter !== undefined) {
    return node.converter.convert(source, node, path);
  }
  const sourceType = source === null ? "null" : Array.isArray(source) ? "array" : typeof source;
  const custom = options.customConverters?.[sourceType];
  if (custom !== undefined) {
    return custom(source, node, path);
  }

  switch (node.shape.kind) {
    case "Scalar":
      return convertScalar(source, node, path);
    case "Struct": {
      const fields = node.shape.fields;
      if (!isPlainObject(source) && !isClassInstance(source)) {
        throw fail(path, `unsupported source kind '${typeof source}' for struct conversion`);
      }
      // preserve the target's prototype when a target object is given
      const result: Record<string, unknown> =
        target !== null && typeof target === "object" && !Array.isArray(target)
          ? (reflectClone(target) as Record<string, unknown>)
          : {};
      const sourceMap = source as Record<string, unknown>;
      if (isPlainObject(source)) {
        // map source (plain object / Value-like): every source key must have a
        // schema field, else the conversion fails (so a set can fall through
        // to the node that actually matches)
        for (const key of Object.keys(sourceMap)) {
          const field = fields.find(([name]) => name === key);
          if (field === undefined) {
            throw fail(path, `schema for field '${key}' not found for struct conversion`);
          }
          const childPath = [...path, { kind: "Key", key } as const];
          try {
            result[key] = convertSchema(sourceMap[key], field[1], result[key], childPath, options, outcome);
          } catch (err) {
            if (!node.nilable) throw err;
          }
        }
      } else {
        // struct source (class instance): convert field by field, matched by
        // name; schema fields absent from the source are skipped
        for (const [fieldName, fieldSchema] of fields) {
          if (!Object.prototype.hasOwnProperty.call(sourceMap, fieldName)) {
            continue;
          }
          const childPath = [...path, { kind: "Key", key: fieldName } as const];
          try {
            result[fieldName] = convertSchema(sourceMap[fieldName], fieldSchema, result[fieldName], childPath, options, outcome);
          } catch (err) {
            if (!node.nilable) throw err;
          }
        }
      }
      return result;
    }
    case "Map": {
      const shape = node.shape;
      const sourceMap = asRecord(source);
      if (sourceMap === undefined) {
        throw fail(path, `unsupported source kind '${typeof source}' for map conversion`);
      }
      const result: Record<string, unknown> = {};
      for (const keyString of Object.keys(sourceMap)) {
        const childPath = [...path, { kind: "Key", key: keyString } as const];
        const entry = shape.entries.find(([k]) => k === keyString);
        const valueSchema = entry !== undefined ? entry[1] : shape.value;
        if (valueSchema === undefined) {
          throw fail(path, `schema for map key '${keyString}' not found`);
        }
        let newKey = keyString;
        if (shape.key !== undefined) {
          newKey = String(convertSchema(keyString, shape.key, undefined, childPath, options, outcome));
        }
        result[newKey] = convertSchema(sourceMap[keyString], valueSchema, undefined, childPath, options, outcome);
      }
      return result;
    }
    case "Sequence": {
      const shape = node.shape;
      if (!Array.isArray(source)) {
        throw fail(path, "source is not a sequence");
      }
      const result: unknown[] = [];
      for (let index = 0; index < source.length; index++) {
        const childPath = [...path, { kind: "Index", index } as const];
        const item = shape.items.find(([i]) => i === index);
        const elementSchema = item !== undefined ? item[1] : shape.elements;
        if (elementSchema === undefined) {
          throw fail(path, "no schema to convert element(s) in sequence found");
        }
        result.push(convertSchema(source[index], elementSchema, undefined, childPath, options, outcome));
      }
      return result;
    }
    case "Indirection":
      // TS has no pointer wrappers: convert through to the inner schema
      return convertSchema(source, node.shape.inner, target, path, options, outcome);
    case "Enum": {
      // TS has no runtime enums; an enum-shaped source is { variant, payload? }.
      // A bare (non-object) source wraps into the "Some" variant — the
      // Option-like case, e.g. a plain string against Option<string>.
      const sourceMap = asRecord(source);
      if (sourceMap !== undefined && typeof sourceMap.variant === "string") {
        const active = sourceMap.variant;
        const variant = node.shape.variants.find(([name]) => name === active);
        if (variant === undefined) {
          throw fail(path, `no schema for variant '${active}'`);
        }
        const [, variantSchema] = variant;
        if (variantSchema === undefined) return { variant: active };
        return { variant: active, payload: convertSchema(sourceMap.payload, variantSchema, undefined, path, options, outcome) };
      }
      const some = node.shape.variants.find(([name]) => name === "Some");
      if (some !== undefined && some[1] !== undefined) {
        return { variant: "Some", payload: convertSchema(source, some[1], undefined, path, options, outcome) };
      }
      throw fail(path, "source is not an enum");
    }
  }
}

function convertSet(
  source: unknown,
  set: { defaultKey?: string; nodes: [string, SchemaNode][] },
  target: unknown,
  path: SchemaPath,
  options: ConversionOptions,
  outcome: ConversionOutcome,
): unknown {
  if (set.nodes.length === 0) {
    throw fail(path, "no schema nodes found");
  }
  let lastErr: unknown;
  const tryNode = (key: string, node: SchemaNode): unknown => {
    try {
      const value = convertNodeWith(source, node, target, path, options, outcome);
      outcome.validSetKeys.push(key);
      return value;
    } catch (err) {
      lastErr = err;
      throw err;
    }
  };
  const ordered = [...set.nodes].sort(([a], [b]) =>
    a === set.defaultKey ? -1 : b === set.defaultKey ? 1 : 0,
  );
  for (const [key, node] of ordered) {
    try {
      return tryNode(key, node);
    } catch {
      // try the next node
    }
  }
  throw lastErr instanceof Error ? lastErr : fail(path, "source not convertible against any schema node");
}

/** Source as a plain record (plain object or class instance), else undefined. */
function asRecord(source: unknown): Record<string, unknown> | undefined {
  if (source === null || typeof source !== "object" || Array.isArray(source)) return undefined;
  return source as Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// scalar coercions (port of convertTo{Int,Uint,Float,String,Bool})
// ---------------------------------------------------------------------------

function convertScalar(source: unknown, node: SchemaNode, path: SchemaPath): unknown {
  const failMsg = (message: string) => fail(path, message);
  switch (node.typeName) {
    case "number":
    case "float":
    case "f64":
    case "int":
    case "i64":
      return toNumber(source, failMsg);
    case "bigint":
      return toBigint(source, failMsg);
    case "boolean":
    case "bool":
      return toBool(source, failMsg);
    case "string":
      return toStr(source, failMsg);
    case "null":
      return null;
    default:
      throw failMsg(`unsupported scalar target type '${node.typeName}'`);
  }
}

/** Source as number: numbers, bools (0/1), numeric strings. */
export function toNumber(source: unknown, failMsg: (m: string) => SchemaError): number {
  if (typeof source === "number") return source;
  if (typeof source === "boolean") return source ? 1 : 0;
  if (typeof source === "bigint") return Number(source);
  if (typeof source === "string") {
    const parsed = Number(source);
    if (Number.isNaN(parsed) || source.trim() === "") throw failMsg(`cannot parse '${source}' as number`);
    return parsed;
  }
  throw failMsg(`unsupported source type '${typeof source}' for number conversion`);
}

export function toBigint(source: unknown, failMsg: (m: string) => SchemaError): bigint {
  if (typeof source === "bigint") return source;
  if (typeof source === "number") return BigInt(Math.trunc(source));
  if (typeof source === "boolean") return source ? 1n : 0n;
  if (typeof source === "string") {
    try {
      return BigInt(source);
    } catch {
      throw failMsg(`cannot parse '${source}' as bigint`);
    }
  }
  throw failMsg(`unsupported source type '${typeof source}' for bigint conversion`);
}

/** Source as bool: bools, numbers (!= 0), strings ("true"/"false"). */
export function toBool(source: unknown, failMsg: (m: string) => SchemaError): boolean {
  if (typeof source === "boolean") return source;
  if (typeof source === "number") return source !== 0;
  if (typeof source === "bigint") return source !== 0n;
  if (typeof source === "string") {
    if (source === "true") return true;
    if (source === "false") return false;
    throw failMsg(`cannot parse '${source}' as bool`);
  }
  throw failMsg(`unsupported source type '${typeof source}' for bool conversion`);
}

/** Source as string: any scalar renders directly. */
export function toStr(source: unknown, failMsg: (m: string) => SchemaError): string {
  if (typeof source === "string") return source;
  if (typeof source === "number" || typeof source === "bigint" || typeof source === "boolean") {
    return String(source);
  }
  if (source === null) return "null";
  throw failMsg(`unsupported source type '${typeof source}' for string conversion`);
}
