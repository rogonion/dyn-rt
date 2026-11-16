// Core domain (port of libs/rust/metadatamodel/src/core/domain.rs): the
// metadata model as a JSON-like structure and its vocabulary.
//
// The metadata model is NOT a concrete class: it is a shared `Value` tree
// (objects/arrays/scalars), so it is extensible (unknown properties are
// carried along untouched) and can itself be traversed with the path /
// manipulation modules. A *group* is an object; its fields live under the
// structural key FIELDS_KEY, which is ALWAYS an array whose first element is
// the object mapping field-name -> field/group. Field props are likewise
// always an array (one props object), so a field's `group:path_key` template
// carries `[*]` all the way to the end, e.g. `$['group:fields'][*].Bio[*]`.
// The same template fetches the field/group props (schema path, with indices
// substituted) and is post-processed into the actual path to data (see
// application.ts).

import type { Value } from "@dyn-rt/shared";

/** Structural key holding a group's fields (always an array in the model).
 * Namespaced like the other keys, so templates use the bracket form:
 * `$['group:fields'][*].Bio[*]` (TS note: the "group" here is the metadata
 * model's group concept, not a language class). */
export const FIELDS_KEY = "group:fields";

/** Property keys, namespaced `<section>:<key>` (refinement of go's flat
 * `FieldGroup*`/`Field*`/`Group*`/`Database*` names). Unknown sections are
 * simply carried along — the model is extensible by construction. */
export const props = {
  // group section
  GROUP_PATH_KEY: "group:path_key",
  GROUP_NAME: "group:name",
  GROUP_DESCRIPTION: "group:description",
  GROUP_MAX_ENTRIES: "group:max_entries",
  GROUP_IS_PRIMARY_KEY: "group:is_primary_key",
  GROUP_READ_ORDER_OF_FIELDS: "group:read_order_of_fields",
  GROUP_TYPE_ANY: "group:type_any",
  GROUP_CARDINALITY: "group:cardinality",

  // field section
  FIELD_DATA_TYPE: "field:data_type",
  FIELD_UI: "field:ui",
  FIELD_CARDINALITY: "field:cardinality",
  FIELD_PLACEHOLDER: "field:placeholder",
  FIELD_DEFAULT_VALUE: "field:default_value",
  FIELD_DATETIME_FORMAT: "field:datetime_format",
  FIELD_SELECT_OPTIONS: "field:select_options",
  FIELD_CHECKBOX_VALUE_IF_TRUE: "field:checkbox_value_if_true",
  FIELD_CHECKBOX_VALUE_IF_FALSE: "field:checkbox_value_if_false",
  FIELD_CHECKBOX_VALUES_USE_IN_VIEW: "field:checkbox_values_use_in_view",
  FIELD_CHECKBOX_VALUES_USE_IN_STORAGE: "field:checkbox_values_use_in_storage",

  // view section
  VIEW_TABLE_LOCK_COLUMN: "view:table_lock_column",
  VIEW_VALUES_IN_SEPARATE_COLUMNS: "view:values_in_separate_columns",
  VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS: "view:max_values_in_separate_columns",
  VIEW_SEPARATE_COLUMNS_HEADER_FORMAT: "view:separate_columns_header_format",
  VIEW_SEPARATE_COLUMNS_HEADER_INDEX: "view:separate_columns_header_index",
  VIEW_MULTIPLE_VALUES_JOIN_SYMBOL: "view:multiple_values_join_symbol",
  VIEW_DISABLE: "view:disable",
  VIEW_TABLE_IN_2D: "view:table_in_2d",
  VIEW_COLUMN_POSITION: "view:column_position",
  VIEW_POSITION_BEFORE: "view:position_before",

  // input/edit section
  INPUT_DISABLE: "input:disable",
  INPUT_PLACEHOLDER: "input:placeholder",
  EDIT_DISABLE_PROPERTIES: "edit:disable_properties",

  // query section
  QUERY_DISABLE_CONDITIONS_EDIT: "query:disable_conditions_edit",
  QUERY_ADD_FULL_TEXT_SEARCH_BOX: "query:add_full_text_search_box",

  // extract section
  EXTRACT_AS_SINGLE_FIELD: "extract:as_single_field",

  // database section
  DB_FIELD_ADD_TO_FULL_TEXT_SEARCH_INDEX: "db:field_add_to_full_text_search_index",
  DB_SKIP_DATA_EXTRACTION: "db:skip_data_extraction",
  DB_TABLE_COLLECTION_UID: "db:table_collection_uid",
  DB_TABLE_COLLECTION_NAME: "db:table_collection_name",
  DB_TABLE_NAME: "db:table_name",
  DB_COLUMN_NAME: "db:column_name",
  DB_FIELD_VALUE_TYPE: "db:field_value_type",
  DB_JOIN_DEPTH: "db:join_depth",
  DB_DISTINCT: "db:distinct",
  DB_SORT_BY_ASC: "db:sort_by_asc",
  DB_LIMIT: "db:limit",
  DB_OFFSET: "db:offset",
} as const;

/** Whether a field/group's VALUES are a single value or a list.
 *
 * Default is `"array"` (go compatibility). Choosing `"single"` is an
 * explicit, committed choice: every value of the field/group must be single —
 * if the actual data contradicts the declaration, that is a
 * `MetadatamodelError` cardinality violation, never a dynamic exception. */
export type Cardinality = "single" | "array";

export const DEFAULT_CARDINALITY: Cardinality = "array";

/** Parses the property value (`"single"`/`"object"` -> single,
 * `"array"`/`"list"` -> array). */
export function parseCardinality(value: string): Cardinality | undefined {
  if (value === "single" || value === "object") return "single";
  if (value === "array" || value === "list") return "array";
  return undefined;
}

/** The metadata model document: a JSON-like `Value` tree. Not a concrete
 * class — see the module docs. */
export type MetadataModel = Value;
