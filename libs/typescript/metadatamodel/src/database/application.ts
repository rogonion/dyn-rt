// Database subdomain application use cases (port of
// libs/rust/metadatamodel/src/database/application.rs, itself ported from
// go-metadatamodel/database): extract the columns of one database table
// collection from a metadata model (`getColumnFields`), and get/set/delete
// field values in source data through those columns (`FieldValue`).
//
// The `db:*` properties drive the mapping:
// - `db:table_collection_uid` or (`db:join_depth` + `db:table_collection_name`)
//   select which table collection's fields are collected,
// - `db:column_name` names the column a field maps to,
// - each field's `group:path_key` template is post-processed (core's
//   `dataPath`) into the actual path to the field's data.

import { DynObject } from "@dyn-rt/manipulation";
import type { RecursiveDescentSegments } from "@dyn-rt/path";
import { reflectRef } from "@dyn-rt/reflect";
import type { Schema } from "@dyn-rt/schema";
import type { Value } from "@dyn-rt/shared";
import {
  cardinalityOf,
  dataPath,
  fieldsOf,
  isGroup,
  parseTemplate,
  pathKeyOf,
  prop,
  propsOf,
  readOrderOf,
} from "../core/index.js";
import { FIELDS_KEY, props, type MetadataModel } from "../core/domain.js";
import { MetadatamodelError } from "../error.js";
import { PropMatch } from "../fieldcolumns/index.js";
import { ColumnFields } from "./domain.js";

// ---------------------------------------------------------------------------
// getColumnFields
// ---------------------------------------------------------------------------

/** Options for `getColumnFields` (go's `GetColumnFields` builder fields). */
export interface GetColumnFieldsOptions {
  /** Select by `db:table_collection_uid`. */
  tableCollectionUid?: string;
  /** Select by `db:join_depth` (requires `tableCollectionName` too). */
  joinDepth?: number;
  /** Select by `db:table_collection_name` (requires `joinDepth` too). */
  tableCollectionName?: string;
  /** Skip a field/group whose props match one of the entries (collection
   * only — descent into the group's fields still happens). */
  skip?: PropMatch;
  /** Collect only field/groups whose props match one of the entries. */
  add?: PropMatch;
}

/** Extracts the database columns of one table collection from the metadata
 * model (go's `GetColumnFields.Get`). Requires `tableCollectionUid` OR both
 * `joinDepth` and `tableCollectionName`. */
export function getColumnFields(
  model: MetadataModel,
  options: GetColumnFieldsOptions,
): ColumnFields {
  if (
    options.tableCollectionUid === undefined &&
    (options.joinDepth === undefined || options.tableCollectionName === undefined)
  ) {
    throw MetadatamodelError.getColumnFields(
      "db:table_collection_uid or db:join_depth and db:table_collection_name is required",
    );
  }

  const out = new ColumnFields();
  walkFields(model, options, out);
  return out;
}

/** Recursive walk over every field/group of `model` in read order (go's
 * `iter.ForEach` as used by `GetColumnFields.Get`). */
function walkFields(
  model: MetadataModel,
  options: GetColumnFieldsOptions,
  out: ColumnFields,
): void {
  const fieldsObj = fieldsOf(model);
  const order = readOrderOf(model);

  for (const key of order) {
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(
        `field ${JSON.stringify(key)} not found in ${JSON.stringify(FIELDS_KEY)}[0]`,
      );
    }
    const fieldProps = propsOf(entry);

    // skip / add affect collection only — descent into groups continues
    const skipped =
      options.skip !== undefined && options.skip.isValid() && options.skip.firstMatch(fieldProps);
    const notAdded =
      options.add !== undefined && options.add.isValid() && !options.add.firstMatch(fieldProps);
    if (skipped || notAdded) {
      if (isGroup(fieldProps)) {
        walkFields(fieldProps, options, out);
      }
      continue;
    }

    // the table collection criterion
    let matches: boolean;
    if (options.tableCollectionUid !== undefined) {
      const uid = prop(fieldProps, props.DB_TABLE_COLLECTION_UID);
      matches = typeof uid === "string" && uid === options.tableCollectionUid;
    } else {
      const depth = prop(fieldProps, props.DB_JOIN_DEPTH);
      const depthMatches =
        typeof depth === "number" && depth === (options.joinDepth ?? Number.MIN_SAFE_INTEGER);
      const name = prop(fieldProps, props.DB_TABLE_COLLECTION_NAME);
      const nameMatches =
        typeof name === "string" && name === (options.tableCollectionName ?? undefined);
      matches = depthMatches && nameMatches;
    }
    if (!matches) {
      // non-matching subtree: do not descend into it
      continue;
    }

    if (isGroup(fieldProps)) {
      walkFields(fieldProps, options, out);
      continue;
    }

    const columnProp = prop(fieldProps, props.DB_COLUMN_NAME);
    const columnName = typeof columnProp === "string" ? columnProp : "";
    if (columnName === "") {
      throw MetadatamodelError.getColumnFields(
        `db:column_name not found in field/group property ${JSON.stringify(key)}`,
      );
    }
    if (out.get(columnName) !== undefined) {
      throw MetadatamodelError.getColumnFields(`duplicate db:column_name '${columnName}' found`);
    }
    out.readOrder.push(columnName);
    out.fields.push([columnName, fieldProps]);
  }
}

// ---------------------------------------------------------------------------
// field value (get / set / delete in source data through the columns)
// ---------------------------------------------------------------------------

/** Builds the path segments to a column's data in the source: the field's
 * `group:path_key` template post-processed by core's `dataPath`, with
 * `suffixPath` appended (go's `FieldValue.getJsonPathToValue`).
 *
 * When `sourceIsArray` is true the source root is itself a sequence, so the
 * first index addresses the root entry and the remaining indices the
 * template's wildcards. */
export function fieldValuePath(
  columnFields: ColumnFields,
  columnName: string,
  suffixPath: string,
  indices: number[],
  sourceIsArray: boolean,
): RecursiveDescentSegments {
  if (columnName === "") {
    throw MetadatamodelError.fieldValue("column field name is empty");
  }
  const fieldProps = columnFields.get(columnName);
  if (fieldProps === undefined) {
    throw MetadatamodelError.fieldValue(
      `field with db:column_name '${columnName}' not found`,
    );
  }
  const template = pathKeyOf(fieldProps);
  const cardinality = cardinalityOf(fieldProps);

  const rootIndex = sourceIsArray ? (indices[0] ?? 0) : undefined;
  const wildcardIndices = sourceIsArray ? indices.slice(1) : indices;

  let path = dataPath(template, wildcardIndices, cardinality);
  if (suffixPath !== "") {
    path = `${path}.${suffixPath}`;
  }
  const segments = parseTemplate(path);

  if (rootIndex !== undefined) {
    // source root is a sequence: address the root entry right after `$`
    const firstGroup = segments[0];
    if (firstGroup !== undefined) {
      firstGroup.splice(1, 0, {
        accessStrategy: "Positional",
        selector: { kind: "Index", index: rootIndex },
      });
    }
  }
  return segments;
}

/** Get/set/delete field values in `source` using the extracted columns (go's
 * `FieldValue`). `sourceIsArray` is true when the source root is a sequence —
 * detected automatically in the constructor (like go's `SetSourceData`). */
export class FieldValue {
  /** Whether the source root is a sequence (auto-detected). */
  readonly sourceIsArray: boolean;

  constructor(
    /** The source data to manipulate, wrapped in the DynObject entrypoint. */
    public readonly obj: DynObject,
    /** The extracted database columns (from `getColumnFields`). */
    public readonly columnFields: ColumnFields,
  ) {
    this.sourceIsArray = reflectRef(obj.getSource()) === "Sequence";
  }

  private pathSegments(
    columnName: string,
    suffixPath: string,
    indices: number[],
  ): RecursiveDescentSegments {
    return fieldValuePath(this.columnFields, columnName, suffixPath, indices, this.sourceIsArray);
  }

  /** Gets the value(s) of column `columnName` (go's `FieldValue.Get`).
   * Returns one entry per path match. */
  get(columnName: string, suffixPath: string, indices: number[]): unknown[] {
    const segments = this.pathSegments(columnName, suffixPath, indices);
    try {
      return this.obj.get(segments);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }
  }

  /** Sets `value` at column `columnName` (go's `FieldValue.Set`). A schema
   * guides coercion and creation of missing intermediate structures.
   * Returns how many values were set. */
  set(
    columnName: string,
    value: unknown,
    suffixPath: string,
    indices: number[],
    schema?: Schema,
  ): number {
    const segments = this.pathSegments(columnName, suffixPath, indices);
    try {
      return this.obj.setWithSchema(segments, value, schema);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }
  }

  /** Deletes the value(s) of column `columnName` (go's `FieldValue.Delete`).
   * Returns how many values were deleted. */
  delete(columnName: string, suffixPath: string, indices: number[]): number {
    const segments = this.pathSegments(columnName, suffixPath, indices);
    try {
      return this.obj.delete(segments);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }
  }
}
