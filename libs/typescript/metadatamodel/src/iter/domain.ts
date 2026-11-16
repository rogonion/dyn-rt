// Iter subdomain domain (port of
// libs/rust/metadatamodel/src/iter/domain.rs): control-flow decisions
// returned by the walk callbacks (port of go-metadatamodel/iter's callback
// return tuples).

/** What `forEach`'s callback decides for the current field/group (go's
 * `ForeachCallback` returns). */
export interface Visit {
  /** `true` terminates the whole walk. */
  terminate: boolean;
  /** `true` skips descending into the field/group's nested fields. */
  skipFields: boolean;
}

export const Visit = {
  /** Continue the walk, descending into groups. */
  CONTINUE: { terminate: false, skipFields: false } satisfies Visit,
  /** Terminate the whole walk. */
  TERMINATE: { terminate: true, skipFields: false } satisfies Visit,
};

/** What `filter`'s callback decides for the current field/group (go's
 * `FilterCallback` returns). */
export interface FilterDecision {
  /** `false` removes the field/group from the model. */
  retain: boolean;
  /** `true` skips filtering the field/group's nested fields. */
  skipFields: boolean;
}

export const FilterDecision = {
  /** Keep the field/group and filter its nested fields too. */
  RETAIN: { retain: true, skipFields: false } satisfies FilterDecision,
  /** Remove the field/group. */
  REMOVE: { retain: false, skipFields: false } satisfies FilterDecision,
};
