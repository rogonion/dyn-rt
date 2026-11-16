// fieldcolumns application (port of
// libs/rust/metadatamodel/src/fieldcolumns/application.rs): extract the
// metadata model's fields as flat 2D table columns (port of
// go-metadatamodel/fieldcolumns extraction.go), plus the skip operation and
// the group column indexes used by the flattener/unflattener.
//
// Walk order is `group:read_order_of_fields` (default: the fields object's
// key order). A group is recursed into, unless `extract:as_single_field` is
// set or it expands into separate columns (`view:values_in_separate_columns`
// with `view:max_values_in_separate_columns` > 0 and no nested groups). A
// field with that view prop expands into one column per value index.

import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { areEqual } from "@dyn-rt/manipulation";
import { schemaAtPath, type Schema } from "@dyn-rt/schema";
import { asObject, type JsonObject, type Value } from "@dyn-rt/shared";
import {
  cardinalityOf,
  containsNestedGroups,
  dataPath,
  fieldsOf,
  isGroup,
  mergeProps,
  nameOf,
  pathKeyOf,
  prop,
  propsOf,
  readOrderOf,
  setProp,
} from "../core/index.js";
import { FIELDS_KEY, props, type MetadataModel } from "../core/domain.js";
import { MetadatamodelError } from "../error.js";
import {
  ColumnFields,
  columnKeyOf,
  fieldColumnPosition,
  type ColumnField,
  type FieldColumnPosition,
} from "./domain.js";

// ---------------------------------------------------------------------------
// property matching (go's FieldGroupPropertiesMatch, deep-equal semantics)
// ---------------------------------------------------------------------------

/** A set of property entries that must all be present and deep-equal for a
 * match. matchingProps() returns the matched subset (merged into nested
 * fields by the extraction). */
export class PropMatch {
  constructor(public entries: [string, Value][]) {}

  isValid(): boolean {
    return this.entries.length > 0;
  }

  /** Whether every entry is present in `props` and deep-equal. */
  firstMatch(propsValue: Value): boolean {
    return this.entries.every(([key, expected]) => {
      const got = prop(propsValue, key);
      return got !== undefined && areEqual(got, expected);
    });
  }

  /** The subset of `entries` present and equal in `props`. */
  matchingProps(propsValue: Value): Value {
    const out: [string, Value][] = [];
    for (const [key, expected] of this.entries) {
      const got = prop(propsValue, key);
      if (got !== undefined && areEqual(got, expected)) {
        out.push([key, expected]);
      }
    }
    return Object.fromEntries(out);
  }
}

// ---------------------------------------------------------------------------
// extraction
// ---------------------------------------------------------------------------

/** Options for extract(). */
export interface ExtractOptions {
  /** Schema of the data the model describes; when set, each column carries
   * the schema at its data path. */
  schema?: Schema;
  /** When valid, matching fields/groups get the match merged into their
   * nested fields (go's skip/add propagation). */
  skip?: PropMatch;
  add?: PropMatch;
}

/** Extracts the model's fields as columns. `model` is the root group's props
 * object. */
export function extract(model: MetadataModel, options: ExtractOptions = {}): ColumnFields {
  const out = new ColumnFields();
  const state: ExtractState = { matching: {}, nextPositionCell: {} };
  recursiveExtract(model, state, out, options);
  return out;
}

/** Mutable walk state threaded through the recursion (TS has no &mut locals,
 * so the two mutable slots are bundled in one state object). */
interface ExtractState {
  matching: Value;
  nextPositionCell: { value?: FieldColumnPosition };
}

function recursiveExtract(
  group: Value,
  state: ExtractState,
  out: ColumnFields,
  options: ExtractOptions,
): void {
  const fieldsObj = fieldsOf(group);
  const readOrder = readOrderOf(group);

  for (const keySuffix of readOrder) {
    const entry = fieldsObj[keySuffix];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(`field ${JSON.stringify(keySuffix)} not found in ${FIELDS_KEY}`);
    }
    const fgProps = propsOf(entry);

    // merge skip/add matches into the inherited matching props
    const fgMatching: Value = structuredCloneish(state.matching);
    if (options.skip !== undefined && options.skip.isValid()) {
      mergeProps(fgMatching, options.skip.matchingProps(fgProps));
    }
    if (options.add !== undefined && options.add.isValid()) {
      mergeProps(fgMatching, options.add.matchingProps(fgProps));
    }

    // position override from view:column_position, else inherited
    const position = extractColumnPosition(fgProps);
    if (position !== undefined) {
      state.nextPositionCell.value = position;
    }

    const pathKey = pathKeyOf(fgProps);

    if (isGroup(fgProps)) {
      const asSingle = prop(fgProps, props.EXTRACT_AS_SINGLE_FIELD) === true;
      const maxColumns = maxFlatColumns(fgProps);
      if (!asSingle && maxColumns > 0 && !containsNestedGroups(fgProps)) {
        // group expanded into separate columns: each sub-field becomes a
        // column per value index
        extractGroupAsColumns(fgProps, pathKey, fgMatching, state, out, options, maxColumns);
        continue;
      }

      // recurse into the group
      recursiveExtract(fgProps, { matching: fgMatching, nextPositionCell: state.nextPositionCell }, out, options);
      continue;
    }

    // plain field (or group as single field)
    const maxColumns = maxFlatColumns(fgProps);
    if (maxColumns > 0) {
      // field expanded into separate columns, one per value index
      for (let columnIndex = 0; columnIndex < maxColumns; columnIndex++) {
        const newField: Value = structuredCloneish(fgProps);
        mergeProps(newField, fgMatching);
        addFlatColumnContext(newField, columnIndex);
        const pos = fieldColumnPosition(pathKey, {
          fieldInSeparateColumns: true,
          fieldHeaderIndex: columnIndex,
        });
        appendField(newField, pos, out, options);
        // one reposition instruction per position prop: consumed by the
        // first column appended after it
        const flowing = state.nextPositionCell.value;
        if (flowing !== undefined) {
          state.nextPositionCell.value = undefined;
          setReposition(flowing, out);
        }
      }
      continue;
    }

    // plain field, single column
    const newField: Value = structuredCloneish(fgProps);
    mergeProps(newField, fgMatching);
    const pos = fieldColumnPosition(pathKey);
    appendField(newField, pos, out, options);
    const flowing = state.nextPositionCell.value;
    if (flowing !== undefined) {
      state.nextPositionCell.value = undefined;
      setReposition(flowing, out);
    }
  }
}

/** A non-nested group with separate columns: each sub-field becomes one
 * column per value index, keyed `parent[i].suffix`. */
function extractGroupAsColumns(
  fgProps: Value,
  pathKey: string,
  fgMatching: Value,
  state: ExtractState,
  out: ColumnFields,
  options: ExtractOptions,
  maxColumns: number,
): void {
  const nestedObj = fieldsOf(fgProps);
  const nestedOrder = readOrderOf(fgProps);
  for (let columnIndex = 0; columnIndex < maxColumns; columnIndex++) {
    for (const nestedSuffix of nestedOrder) {
      const nestedEntry = nestedObj[nestedSuffix];
      if (nestedEntry === undefined) {
        throw MetadatamodelError.invalidModel(`nested field ${JSON.stringify(nestedSuffix)} not found`);
      }
      const nestedProps = propsOf(nestedEntry);
      const newField: Value = structuredCloneish(nestedProps);
      mergeProps(newField, fgMatching);
      addFlatColumnContext(newField, columnIndex);
      const nestedKey = pathKeyOf(nestedProps);
      const pos = fieldColumnPosition(nestedKey, {
        groupInSeparateColumns: true,
        groupHeaderIndex: columnIndex,
        groupParentPathKey: pathKey,
        fieldKeySuffix: nestedSuffix,
      });
      appendField(newField, pos, out, options);
      const flowing = state.nextPositionCell.value;
      if (flowing !== undefined) {
        state.nextPositionCell.value = undefined;
        setReposition(flowing, out);
      }
    }
  }
}

/** Records `position` as a reposition instruction with source = the column
 * just appended (go's setRepositionForFieldColumn). */
function setReposition(position: FieldColumnPosition, out: ColumnFields): void {
  position.sourceIndex = Math.max(out.readOrder.length - 1, 0);
  out.repositionInstructions.push(position);
}

/** Maximum number of separate columns for a field/group: the
 * `view:max_values_in_separate_columns` value when
 * `view:values_in_separate_columns` is true (groups containing nested groups
 * force -1), else -1. A `single` cardinality caps it at 1. */
export function maxFlatColumns(fgProps: Value): number {
  let maxColumns = -1;
  const inSeparate = prop(fgProps, props.VIEW_VALUES_IN_SEPARATE_COLUMNS) === true;
  if (inSeparate) {
    if (isGroup(fgProps) && containsNestedGroups(fgProps)) {
      return -1;
    }
    const value = prop(fgProps, props.VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS);
    if (value !== undefined) {
      if (typeof value !== "number") {
        throw MetadatamodelError.invalidModel(
          `${JSON.stringify(props.VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS)} is not a number`,
        );
      }
      maxColumns = Math.trunc(value);
    }
    if (cardinalityOf(fgProps) === "single" && maxColumns > 1) {
      maxColumns = 1;
    }
  }
  return maxColumns;
}

/** Reads `view:column_position` (`{ group:path_key, view:position_before,
 * view:separate_columns_header_index }`). */
function extractColumnPosition(fgProps: Value): FieldColumnPosition | undefined {
  const value = prop(fgProps, props.VIEW_COLUMN_POSITION);
  const obj = value !== undefined ? asObject(value) : undefined;
  if (obj === undefined) return undefined;
  const pathKey = obj[props.GROUP_PATH_KEY];
  if (typeof pathKey !== "string") return undefined;
  const headerIndex = obj[props.VIEW_SEPARATE_COLUMNS_HEADER_INDEX];
  return fieldColumnPosition(pathKey, {
    positionBefore: obj[props.VIEW_POSITION_BEFORE] === true,
    fieldHeaderIndex: typeof headerIndex === "number" ? Math.trunc(headerIndex) : 0,
  });
}

/** Stamps a separate-column field copy with its header index and name (go's
 * addFlatColumnContext): the `[*]` in the header format is replaced by the
 * 1-based column number, else `"<name> <n>"`. */
function addFlatColumnContext(field: Value, columnIndex: number): void {
  const obj = asObject(field);
  if (obj === undefined) return;
  const headerFormat = obj[props.VIEW_SEPARATE_COLUMNS_HEADER_FORMAT];
  const baseName = nameOf(field, "");
  setProp(field, props.VIEW_SEPARATE_COLUMNS_HEADER_INDEX, columnIndex);
  const newName =
    typeof headerFormat === "string" && headerFormat !== ""
      ? headerFormat.replaceAll("[*]", String(columnIndex + 1))
      : `${baseName} ${columnIndex + 1}`;
  setProp(field, props.GROUP_NAME, newName);
}

function appendField(
  field: Value,
  position: FieldColumnPosition,
  out: ColumnFields,
  options: ExtractOptions,
): void {
  // per-column schema: the schema node at the field's data path
  let schema: Schema | undefined;
  if (options.schema !== undefined) {
    const cardinality = cardinalityOf(field);
    const template = pathKeyOf(field);
    const path = dataPath(template, [], cardinality);
    const segments = parsePath(new RegexPathTokenizer(), path);
    const group = segments[0];
    if (group !== undefined) {
      try {
        schema = schemaAtPath(group, options.schema).node;
      } catch {
        schema = undefined;
      }
    }
  }
  const column: ColumnField = {
    props: field,
    schema,
    indexInReadOrder: out.readOrder.length,
    indexInRepositioned: 0,
    skip: false,
  };
  out.fields.push([columnKeyOf(position), column]);
  out.readOrder.push(position);
  out.current.push(out.readOrder.length - 1);
}

/** Deep clone for Value trees (TS structuredClone loses nothing here, but an
 * explicit clone keeps the Value type and avoids cloning class instances). */
function structuredCloneish(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}

// ---------------------------------------------------------------------------
// group column indexes (used by flattener and unflattener to know which
// columns belong to each group)
// ---------------------------------------------------------------------------

/** Fast lookup index identifying which columns to read at a group (go's
 * `GroupColumnIndexes`). Indexes are into the unskipped current read order. */
export interface GroupColumnIndexes {
  /** Primary key columns. */
  primary: number[];
  /** All columns of the group. */
  all: number[];
}

/** Retrieves the column indexes (primary and all) of `group` within
 * `columnFields` (go's `GroupsColumnsIndexesRetrieval.Get`). */
export function groupColumnIndexes(
  columnFields: ColumnFields,
  group: MetadataModel,
): GroupColumnIndexes {
  const indexes: GroupColumnIndexes = { primary: [], all: [] };
  collectGroupColumnIndexes(columnFields, group, true, false, indexes);
  return indexes;
}

/** The leaf positions a field (or group-as-separate-columns) contributes:
 * one per separate column when `maxFlatColumns > 0`, else a single one. */
function leafPositions(pathKey: string, maxColumns: number): FieldColumnPosition[] {
  const positions: FieldColumnPosition[] = [];
  if (maxColumns > 0) {
    for (let columnIndex = 0; columnIndex < maxColumns; columnIndex++) {
      positions.push(
        fieldColumnPosition(pathKey, { fieldInSeparateColumns: true, fieldHeaderIndex: columnIndex }),
      );
    }
  } else {
    positions.push(fieldColumnPosition(pathKey));
  }
  return positions;
}

function appendColumnIndex(
  columnFields: ColumnFields,
  position: FieldColumnPosition,
  isPrimary: boolean,
  isAll: boolean,
  indexes: GroupColumnIndexes,
): void {
  const field = columnFields.getByKey(columnKeyOf(position));
  if (field === undefined) {
    throw MetadatamodelError.invalidModel(
      `column '${columnKeyOf(position)}' not found in column fields`,
    );
  }
  const unskipped = columnFields.indexInUnskipped(field.indexInReadOrder);
  if (unskipped === undefined) return;
  if (isAll) indexes.all.push(unskipped);
  if (isPrimary) indexes.primary.push(unskipped);
}

function collectGroupColumnIndexes(
  columnFields: ColumnFields,
  group: MetadataModel,
  isAll: boolean,
  forcePrimary: boolean,
  indexes: GroupColumnIndexes,
): void {
  const order = readOrderOf(group);
  const fieldsObj = fieldsOf(group);

  for (const key of order) {
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(`field ${JSON.stringify(key)} not found in ${FIELDS_KEY}[0]`);
    }
    const fieldProps = propsOf(entry);
    const fieldIsPrimary =
      forcePrimary || prop(fieldProps, props.GROUP_IS_PRIMARY_KEY) === true;
    const pathKey = pathKeyOf(fieldProps);

    if (isGroup(fieldProps)) {
      const asSingle = prop(fieldProps, props.EXTRACT_AS_SINGLE_FIELD) === true;
      const maxColumns = maxFlatColumns(fieldProps);
      if (!asSingle && maxColumns > 0 && !containsNestedGroups(fieldProps)) {
        // group-as-separate-columns: leaves per nested field per column
        const nestedOrder = readOrderOf(fieldProps);
        const nestedObj: JsonObject = fieldsOf(fieldProps);
        for (let columnIndex = 0; columnIndex < maxColumns; columnIndex++) {
          for (const nestedSuffix of nestedOrder) {
            const nestedEntry = nestedObj[nestedSuffix];
            if (nestedEntry === undefined) {
              throw MetadatamodelError.invalidModel(`nested field ${JSON.stringify(nestedSuffix)} not found`);
            }
            const nestedKey = pathKeyOf(propsOf(nestedEntry));
            const position = fieldColumnPosition(nestedKey, {
              groupInSeparateColumns: true,
              groupHeaderIndex: columnIndex,
              groupParentPathKey: pathKey,
              fieldKeySuffix: nestedSuffix,
            });
            appendColumnIndex(columnFields, position, fieldIsPrimary, true, indexes);
          }
        }
        continue;
      }
      if (asSingle) {
        // processed as a plain field below
      } else {
        // nested group: contributes no columns itself; its primary keys
        // bubble up when the group is primary
        if (fieldIsPrimary) {
          collectGroupColumnIndexes(columnFields, fieldProps, false, true, indexes);
        }
        continue;
      }
    }

    // plain field (or group as single field)
    const maxColumns = maxFlatColumns(fieldProps);
    for (const position of leafPositions(pathKey, maxColumns)) {
      appendColumnIndex(columnFields, position, fieldIsPrimary, isAll, indexes);
    }
  }
}
