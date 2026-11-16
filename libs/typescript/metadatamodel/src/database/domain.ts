// Database subdomain domain (port of
// libs/rust/metadatamodel/src/database/domain.rs): the columns of one
// database table collection.

import type { Value } from "@dyn-rt/shared";

/** Columns/fields of a particular `db:table_collection_name` at a specific
 * `db:join_depth`, extracted from a metadata model by
 * `getColumnFields` (application.ts).
 *
 * The key of each entry is the field's `db:column_name`. */
export class ColumnFields {
  /** Read order of the column names. */
  readOrder: string[] = [];
  /** The field/group props of each column, keyed by `db:column_name`. */
  fields: [string, Value][] = [];

  /** The field/group props of the column with this `db:column_name`. */
  get(columnName: string): Value | undefined {
    return this.fields.find(([key]) => key === columnName)?.[1];
  }
}
