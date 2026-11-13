//! Iter subdomain domain: control-flow decisions returned by the walk
//! callbacks (port of go-metadatamodel/iter's callback return tuples).

/// What [`crate::iter::application::for_each`]'s callback decides for the
/// current field/group (go's `ForeachCallback` returns).
#[derive(Debug, Clone, Copy, Default)]
pub struct Visit {
    /// `true` terminates the whole walk.
    pub terminate: bool,
    /// `true` skips descending into the field/group's nested fields.
    pub skip_fields: bool,
}

impl Visit {
    /// Continue the walk, descending into groups.
    pub const CONTINUE: Visit = Visit {
        terminate: false,
        skip_fields: false,
    };
    /// Terminate the whole walk.
    pub const TERMINATE: Visit = Visit {
        terminate: true,
        skip_fields: false,
    };
}

/// What [`crate::iter::application::filter`]'s callback decides for the
/// current field/group (go's `FilterCallback` returns).
#[derive(Debug, Clone, Copy, Default)]
pub struct FilterDecision {
    /// `false` removes the field/group from the model.
    pub retain: bool,
    /// `true` skips filtering the field/group's nested fields.
    pub skip_fields: bool,
}

impl FilterDecision {
    /// Keep the field/group and filter its nested fields too.
    pub const RETAIN: FilterDecision = FilterDecision {
        retain: true,
        skip_fields: false,
    };
    /// Remove the field/group.
    pub const REMOVE: FilterDecision = FilterDecision {
        retain: false,
        skip_fields: false,
    };
}
