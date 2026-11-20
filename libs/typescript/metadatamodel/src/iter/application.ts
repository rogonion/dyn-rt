// Iter subdomain application use cases (port of
// libs/rust/metadatamodel/src/iter/application.rs, itself ported from
// go-metadatamodel/iter): recursive walks over the metadata model's
// fields/groups — `forEach` (visit), `filter` (remove fields/groups in
// place), and `map` (replace field/group entries in place).
//
// All three walk each group's fields in read order
// (`group:read_order_of_fields`, defaulting to the fields key order) and
// recurse into groups unless the callback says otherwise. Callbacks receive
// the field/group's PROPS object (the first element of its entry array).
// TS note: Values are mutable object references, so filter/map mutate the
// model in place directly (no &mut plumbing needed).

import { asArray, asObject, get, type Value } from "@dyn-rt/shared";
import { fieldsOf, isGroup, propsOf, readOrderOf } from "../core/index.js";
import { FIELDS_KEY, props, type MetadataModel } from "../core/domain.js";
import { MetadatamodelError } from "../error.js";
import { FilterDecision, Visit } from "./domain.js";

// ---------------------------------------------------------------------------
// for_each
// ---------------------------------------------------------------------------

/** Recursively visits every field/group of `group` in read order (go's
 * `iter.ForEach`). The callback receives the field/group's props object. */
export function forEach(group: MetadataModel, callback: (props: Value) => Visit): void {
  const order = readOrderOf(group);
  const fieldsObj = fieldsOf(group);

  for (const key of order) {
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(`field ${JSON.stringify(key)} not found in ${JSON.stringify(FIELDS_KEY)}[0]`);
    }
    const fieldProps = propsOf(entry);
    const visit = callback(fieldProps);
    if (visit.terminate) {
      return;
    }
    if (isGroup(fieldProps) && !visit.skipFields) {
      forEach(fieldProps, callback);
    }
  }
}

// ---------------------------------------------------------------------------
// filter
// ---------------------------------------------------------------------------

/** Recursively removes fields/groups from `group` in place (go's
 * `iter.Filter`): the callback decides per field/group; a removed entry is
 * deleted from the `group:fields[0]` object and from
 * `group:read_order_of_fields` when that prop is present. */
export function filter(group: MetadataModel, callback: (props: Value) => FilterDecision): void {
  const order = readOrderOf(group);

  for (const key of order) {
    // decide from the current props
    const fieldsObj = fieldsOf(group);
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(`field ${JSON.stringify(key)} not found in ${JSON.stringify(FIELDS_KEY)}[0]`);
    }
    const fieldProps = propsOf(entry);
    const decision = callback(fieldProps);

    if (!decision.retain) {
      removeField(group, key);
      continue;
    }

    if (decision.skipFields) {
      continue;
    }

    // recurse into the group's fields
    if (isGroup(fieldProps)) {
      filter(fieldProps, callback);
    }
  }
}

/** Removes field/group `key` from the group's fields object and from its
 * `group:read_order_of_fields` array when present. */
function removeField(group: MetadataModel, key: string): void {
  const fields = get(group, FIELDS_KEY);
  const fieldsArray = fields !== undefined ? asArray(fields) : undefined;
  const first = fieldsArray?.[0];
  const fieldsObj = first !== undefined ? asObject(first) : undefined;
  if (fieldsObj !== undefined) {
    delete fieldsObj[key];
  }
  const order = get(group, props.GROUP_READ_ORDER_OF_FIELDS);
  const orderArray = order !== undefined ? asArray(order) : undefined;
  if (orderArray !== undefined) {
    const kept = orderArray.filter((entry) => entry !== key);
    orderArray.length = 0;
    orderArray.push(...kept);
  }
}

// ---------------------------------------------------------------------------
// map
// ---------------------------------------------------------------------------

/** Recursively replaces field/group entries in place (go's `iter.Map`): the
 * callback receives the current props object and returns the new ENTRY (the
 * array value stored under the field name in `group:fields[0]`) plus
 * whether to skip the nested fields. */
export function map(group: MetadataModel, callback: (props: Value) => [newEntry: Value, skipFields: boolean]): void {
  const order = readOrderOf(group);

  for (const key of order) {
    // compute the replacement from the current props
    const fieldsObj = fieldsOf(group);
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(`field ${JSON.stringify(key)} not found in ${JSON.stringify(FIELDS_KEY)}[0]`);
    }
    const [newEntry, skipFields] = callback(propsOf(entry));

    // write the new entry back, then recurse into it if it is a group
    fieldsObj[key] = newEntry;
    if (!skipFields) {
      const fieldProps = propsOf(newEntry);
      if (isGroup(fieldProps)) {
        map(fieldProps, callback);
      }
    }
  }
}
