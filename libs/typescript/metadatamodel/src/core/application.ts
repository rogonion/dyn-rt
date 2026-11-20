// Core application use cases (port of libs/rust/metadatamodel/src/core/application.rs,
// itself ported from go-metadatamodel's `core`): property getters over the
// JSON-like model, the templated `group:path_key` post-processing (schema
// path vs data path), and the strict cardinality check.
//
// Templates keep `[*]` all the way to the end (e.g. `$['group:fields'][*].Bio[*]`):
// schemaPath substitutes indices into every wildcard to address the
// field/group props inside the model, and dataPath additionally strips the
// `group:fields[*]` structure segments and resolves the remaining wildcards
// against the data to get the actual path to data.

import { RegexPathTokenizer, parsePath, renderSegments, type PathSegment, type RecursiveDescentSegment, type RecursiveDescentSegments } from "@dyn-rt/path";
import { unwrapAll } from "@dyn-rt/manipulation";
import { asArray, asObject, get, isObject, type JsonObject, type Value } from "@dyn-rt/shared";
import { MetadatamodelError } from "../error.js";
import { DEFAULT_CARDINALITY, FIELDS_KEY, parseCardinality, props, type Cardinality, type MetadataModel } from "./domain.js";

// ---------------------------------------------------------------------------
// property getters
// ---------------------------------------------------------------------------

/** Reads property `key` from a field/group object. */
export function prop(fieldGroup: MetadataModel, key: string): Value | undefined {
  return get(fieldGroup, key);
}

/** The templated JSONPath of a field/group (`group:path_key`). */
export function pathKeyOf(fieldGroup: MetadataModel): string {
  const key = get(fieldGroup, props.GROUP_PATH_KEY);
  if (typeof key === "string" && key !== "") return key;
  throw MetadatamodelError.missingProperty(props.GROUP_PATH_KEY);
}

/** Display name of a field/group: `group:name`, else the last template
 * segment, else `default` (go's GetFieldGroupName). */
export function nameOf(fieldGroup: MetadataModel, defaultValue: string): string {
  const name = get(fieldGroup, props.GROUP_NAME);
  if (typeof name === "string" && name !== "") return name;
  try {
    const segments = parsePath(new RegexPathTokenizer(), pathKeyOf(fieldGroup));
    const lastGroup = segments[segments.length - 1];
    const last = lastGroup?.[lastGroup.length - 1];
    if (last !== undefined && last.selector.kind === "Key") {
      return last.selector.key;
    }
  } catch {
    // fall through to the default
  }
  if (defaultValue === "") return "#unnamed";
  return defaultValue;
}

/** The fields object of a group: `group:fields` is always an array whose
 * first element is the object mapping field-name -> field/group. */
export function fieldsOf(fieldGroup: MetadataModel): JsonObject {
  const fields = get(fieldGroup, FIELDS_KEY);
  if (fields === undefined) {
    throw MetadatamodelError.invalidModel(`group has no ${JSON.stringify(FIELDS_KEY)} array`);
  }
  const array = asArray(fields);
  if (array === undefined) {
    throw MetadatamodelError.invalidModel(`${JSON.stringify(FIELDS_KEY)} is not an array`);
  }
  const first = array[0];
  if (first === undefined) {
    throw MetadatamodelError.invalidModel(`${JSON.stringify(FIELDS_KEY)} array is empty`);
  }
  const object = asObject(first);
  if (object === undefined) {
    throw MetadatamodelError.invalidModel(`${JSON.stringify(FIELDS_KEY)}[0] is not an object`);
  }
  return object;
}

/** The props object of a field: the field entry is always an array whose
 * first element is the props object. */
export function propsOf(field: Value): JsonObject {
  const array = asArray(field);
  if (array === undefined) {
    throw MetadatamodelError.invalidModel("field entry is not an array of props");
  }
  const first = array[0];
  if (first === undefined) {
    throw MetadatamodelError.invalidModel("field props array is empty");
  }
  const object = asObject(first);
  if (object === undefined) {
    throw MetadatamodelError.invalidModel("field props entry is not an object");
  }
  return object;
}

/** Declared cardinality of a field/group: `field:cardinality` or
 * `group:cardinality`, defaulting to array. */
export function cardinalityOf(fieldGroup: MetadataModel): Cardinality {
  for (const key of [props.FIELD_CARDINALITY, props.GROUP_CARDINALITY]) {
    const value = get(fieldGroup, key);
    if (typeof value === "string") {
      const cardinality = parseCardinality(value);
      if (cardinality !== undefined) return cardinality;
    }
  }
  return DEFAULT_CARDINALITY;
}

/** Strict cardinality check: the value's shape must match the declaration —
 * `array` requires an array, `single` requires a non-array. No dynamic
 * exceptions. */
export function checkValueCardinality(value: unknown, cardinality: Cardinality): void {
  // unwrap Some-enum payloads so they count by their inner shape
  const current = unwrapAll(value);
  const isSequence = Array.isArray(current);
  if (cardinality === "array" && !isSequence) {
    throw MetadatamodelError.cardinalityViolation(`declared 'array' but value is ${typeof current} (single)`);
  }
  if (cardinality === "single" && isSequence) {
    throw MetadatamodelError.cardinalityViolation("declared 'single' but value is an array");
  }
}

/** Whether a field/group props object is a group (has a `group:fields`
 * array). go's IsFieldAGroup additionally requires read order; here the
 * fields array alone decides, and read order defaults to the fields' key
 * order (see readOrderOf). */
export function isGroup(fieldGroup: MetadataModel): boolean {
  const fields = get(fieldGroup, FIELDS_KEY);
  const array = fields !== undefined ? asArray(fields) : undefined;
  return array !== undefined && array.length > 0;
}

/** The read order of a group's fields: `group:read_order_of_fields` when
 * present, else the `group:fields[0]` object's key order (Rust refinement —
 * go errors when the property is absent). */
export function readOrderOf(fieldGroup: MetadataModel): string[] {
  const order = get(fieldGroup, props.GROUP_READ_ORDER_OF_FIELDS);
  if (order !== undefined) {
    const array = asArray(order);
    if (array === undefined) {
      throw MetadatamodelError.invalidModel(`${JSON.stringify(props.GROUP_READ_ORDER_OF_FIELDS)} is not an array`);
    }
    return array.map((entry) => {
      if (typeof entry !== "string") {
        throw MetadatamodelError.invalidModel(`${JSON.stringify(props.GROUP_READ_ORDER_OF_FIELDS)} entry is not a string`);
      }
      return entry;
    });
  }
  return Object.keys(fieldsOf(fieldGroup));
}

/** Whether any of the group's fields is itself a group (go's
 * DoesFieldGroupFieldsContainNestedGroupFields). */
export function containsNestedGroups(fieldGroup: MetadataModel): boolean {
  for (const entry of Object.values(fieldsOf(fieldGroup))) {
    try {
      if (isGroup(propsOf(entry))) return true;
    } catch {
      // not a field entry: skip
    }
  }
  return false;
}

/** Sets property `key` on an object value, replacing any existing entry. */
export function setProp(object: Value, key: string, value: Value): void {
  if (!isObject(object)) return;
  object[key] = value;
}

/** Merges `from`'s properties into `into` (right wins), object-to-object.
 * go's MergeRightJsonObjectIntoLeft. */
export function mergeProps(into: Value, from: Value): void {
  const intoObj = asObject(into);
  const fromObj = asObject(from);
  if (intoObj === undefined || fromObj === undefined) return;
  for (const [key, value] of Object.entries(fromObj)) {
    intoObj[key] = value;
  }
}

// ---------------------------------------------------------------------------
// templated group:path_key post-processing
// ---------------------------------------------------------------------------

/** Parses a `group:path_key` template into segments. */
export function parseTemplate(template: string): RecursiveDescentSegments {
  const segments = parsePath(new RegexPathTokenizer(), template);
  if (segments.length === 0 || segments.every((group) => group.length === 0)) {
    throw MetadatamodelError.pathTemplateInvalid(`template ${JSON.stringify(template)} parsed to no segments`);
  }
  return segments;
}

/** Schema path: the template with every `[*]` wildcard replaced, in order,
 * by the given indices — the actual path to the field/group props inside
 * the metadata model. Missing indices default to 0 (go behavior). */
export function schemaPath(template: string, indices: number[]): string {
  const segments = parseTemplate(template);
  let index = 0;
  const out: RecursiveDescentSegments = segments.map((group) =>
    group.map((segment): PathSegment => {
      if (segment.selector.kind === "Wildcard") {
        // go behavior: missing indices default to 0
        const value = indices[index] ?? 0;
        index++;
        return { accessStrategy: "Mixed", selector: { kind: "Index", index: value } };
      }
      return { accessStrategy: "Mixed", selector: segment.selector };
    }),
  );
  return renderSegments(out);
}

/** Data path: post-processes the template into the actual path to DATA.
 *
 * Every structural `group:fields[*]` (key + following wildcard) segment is
 * removed; each remaining `[*]` is replaced in order by the given indices.
 * When `cardinality` is `"single"`, a trailing `[*]` is trimmed instead of
 * indexed (single values have no index). Missing indices default to 0. */
export function dataPath(template: string, indices: number[], cardinality: Cardinality): string {
  const segments = parseTemplate(template);
  let index = 0;
  const lastGroupIndex = segments.length - 1;

  const outGroups: RecursiveDescentSegment[] = [];
  for (let groupIndex = 0; groupIndex < segments.length; groupIndex++) {
    const group = segments[groupIndex]!;
    const lastGroupLen = group.length;
    const newGroup: PathSegment[] = [];
    let skipNextWildcard = false;
    for (let position = 0; position < group.length; position++) {
      const segment = group[position]!;
      if (skipNextWildcard) {
        // the [*] belonging to a group:fields key: drop it too
        skipNextWildcard = false;
        if (segment.selector.kind === "Wildcard") continue;
      }
      if (segment.selector.kind === "Key" && segment.selector.key === FIELDS_KEY) {
        // structural schema segment: drop it and its wildcard
        skipNextWildcard = true;
        continue;
      }
      if (segment.selector.kind === "Wildcard") {
        const isTrailingWildcard = groupIndex === lastGroupIndex && position === lastGroupLen - 1;
        if (isTrailingWildcard && cardinality === "single") {
          continue; // single values have no index: trim the [*]
        }
        // go behavior: missing indices default to 0
        const value = indices[index] ?? 0;
        index++;
        newGroup.push({ accessStrategy: "Positional", selector: { kind: "Index", index: value } });
        continue;
      }
      newGroup.push({ accessStrategy: strategyOf(segment.selector), selector: segment.selector });
    }
    outGroups.push(newGroup);
  }

  // drop groups left empty by the group:fields removal
  const kept = outGroups.filter((group) => group.length > 0);
  if (kept.length === 0) return "$";
  return renderSegments(kept);
}

/** Model-aware data path: walks the template together with the model, so
 * each field/group's DECLARED cardinality decides whether its `[*]` is
 * replaced by the next index (`array`, the default) or dropped (`single`).
 * `indexLastWildcard` controls the trailing wildcard (the leaf field's own
 * value index): `false` reads the whole value (used by the flattener and
 * unflattener for cell/group reads). */
export function dataPathModelAware(template: string, model: MetadataModel, indices: number[], indexLastWildcard: boolean): string {
  const segments = parseTemplate(template);
  const flat = segments.flatMap((group) => group.map((segment) => segment.selector));
  const total = flat.length;

  const outGroup: PathSegment[] = [];
  let index = 0;
  let currentGroup: MetadataModel = model;
  let i = 0;
  while (i < total) {
    const selector = flat[i]!;
    if (selector.kind === "Root") {
      outGroup.push({ accessStrategy: "Mixed", selector: { kind: "Root" } });
      i += 1;
      continue;
    }
    if (selector.kind === "Key" && selector.key === FIELDS_KEY) {
      // structural schema segment: drop it and its wildcard
      if (i + 1 < total && flat[i + 1]!.kind === "Wildcard") {
        i += 2;
      } else {
        i += 1;
      }
      continue;
    }
    if (selector.kind === "Key") {
      const key = selector.key;
      // find the field/group in the current group to learn its cardinality
      let entry: Value | undefined;
      try {
        entry = fieldsOf(currentGroup)[key];
      } catch {
        entry = undefined;
      }
      let fieldProps: JsonObject | undefined;
      if (entry !== undefined) {
        try {
          fieldProps = propsOf(entry);
        } catch {
          fieldProps = undefined;
        }
      }
      outGroup.push({ accessStrategy: "Named", selector: { kind: "Key", key } });
      if (i + 1 < total && flat[i + 1]!.kind === "Wildcard") {
        const lastWildcard = i + 2 >= total;
        const indexIt = !lastWildcard || indexLastWildcard;
        if (indexIt && (fieldProps === undefined || cardinalityOf(fieldProps) === "array")) {
          const value = indices[index] ?? 0;
          index++;
          outGroup.push({
            accessStrategy: "Positional",
            selector: { kind: "Index", index: value },
          });
        }
        // single cardinality (or dropped last wildcard): no segment
        i += 2;
      } else {
        i += 1;
      }
      if (fieldProps !== undefined) currentGroup = fieldProps;
      continue;
    }
    if (selector.kind === "Wildcard") {
      // stray wildcard not owned by a model field: index it
      const lastWildcard = i + 1 >= total;
      if (!lastWildcard || indexLastWildcard) {
        const value = indices[index] ?? 0;
        index++;
        outGroup.push({ accessStrategy: "Positional", selector: { kind: "Index", index: value } });
      }
      i += 1;
      continue;
    }
    outGroup.push({ accessStrategy: strategyOf(selector), selector });
    i += 1;
  }

  return renderSegments([outGroup]);
}

/** Data path with wildcards KEPT as `[*]` (no index substitution): strips
 * the structural `group:fields[*]` pairs like dataPath but leaves every
 * remaining wildcard in place. Used by the filter subdomain to iterate all
 * values of a field (go's JsonPathToValue with
 * `replaceArrayPathPlaceholderWithActualIndexes=false`). */
export function dataPathKeepWildcards(template: string): string {
  const segments = parseTemplate(template);

  const outGroups: RecursiveDescentSegment[] = [];
  for (const group of segments) {
    const newGroup: PathSegment[] = [];
    let skipNextWildcard = false;
    for (const segment of group) {
      if (skipNextWildcard) {
        // the [*] belonging to a group:fields key: drop it too
        skipNextWildcard = false;
        if (segment.selector.kind === "Wildcard") continue;
      }
      if (segment.selector.kind === "Key" && segment.selector.key === FIELDS_KEY) {
        skipNextWildcard = true;
        continue;
      }
      newGroup.push({ accessStrategy: strategyOf(segment.selector), selector: segment.selector });
    }
    outGroups.push(newGroup);
  }

  const kept = outGroups.filter((group) => group.length > 0);
  if (kept.length === 0) return "$";
  return renderSegments(kept);
}

function strategyOf(selector: PathSegment["selector"]): PathSegment["accessStrategy"] {
  if (selector.kind === "Key") return "Named";
  if (selector.kind === "Index" || selector.kind === "Slice") return "Positional";
  return "Mixed";
}
