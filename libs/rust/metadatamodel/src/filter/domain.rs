//! Filter subdomain domain: the query-condition vocabulary, the logical
//! operator, and the condition-processor port (go's `FilterContext` /
//! `ConditionTrue`).

use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;

use crate::error::MetadatamodelError;

/// Keys inside a filter condition's data object (go's `FilterCondition*`).
pub mod condition_props {
    pub const VALUE: &str = "Value";
    pub const VALUES: &str = "Values";
    pub const DATE_TIME_FORMAT: &str = "DateTimeFormat";
    pub const CASE_INSENSITIVE: &str = "CaseInsensitive";
    pub const ASSUMED_FIELD_TYPE: &str = "AssumedFieldType";
}

/// Keys of a query condition object (go's `QueryCondition*` / `QuerySection*`).
pub mod query {
    pub const TYPE: &str = "Type";
    pub const NEGATE: &str = "Negate";
    pub const VALUE: &str = "Value";
    pub const LOGICAL_OPERATOR: &str = "LogicalOperator";
    pub const TYPE_LOGICAL_OPERATOR: &str = "LogicalOperator";
    pub const TYPE_FIELD_GROUP: &str = "FieldGroup";
    pub const AND: &str = "And";
    pub const OR: &str = "Or";
}

/// The filter condition names — keys that map to condition processors
/// (go's `FilterCondition*`).
pub mod condition {
    pub const NO_OF_ENTRIES_GREATER_THAN: &str = "NoOfEntriesGreaterThan";
    pub const NO_OF_ENTRIES_LESS_THAN: &str = "NoOfEntriesLessThan";
    pub const NO_OF_ENTRIES_EQUAL_TO: &str = "NoOfEntriesEqualTo";
    pub const GREATER_THAN: &str = "GreaterThan";
    pub const LESS_THAN: &str = "LessThan";
    pub const EQUAL_TO: &str = "EqualTo";
    pub const BEGINS_WITH: &str = "BeginsWith";
    pub const ENDS_WITH: &str = "EndsWith";
    pub const CONTAINS: &str = "Contains";
}

/// Assumed field types in a filter condition (go's `core.FieldType*`).
pub mod field_type {
    pub const TEXT: &str = "Text";
    pub const NUMBER: &str = "Number";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const ANY: &str = "Any";
}

/// Datetime format granularity keys (go's `core.FieldDatetimeFormat*`).
pub mod datetime_format {
    pub const YYYYMMDDHHMM: &str = "yyyy-mm-dd hh:mm";
    pub const YYYYMMDD: &str = "yyyy-mm-dd";
    pub const YYYYMM: &str = "yyyy-mm";
    pub const YYYY: &str = "yyyy";
    pub const MM: &str = "mm";
    pub const HHMM: &str = "hh:mm";
}

/// The logical operator combining sub-conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOperator {
    And,
    Or,
}

impl LogicalOperator {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            query::AND => Some(LogicalOperator::And),
            query::OR => Some(LogicalOperator::Or),
            _ => None,
        }
    }
}

/// Context available to condition processors (go's `FilterContext`).
pub trait FilterContext {
    /// Retrieves the field/group props at a schema path key inside the
    /// metadata model.
    fn get_field_group_by_path_key(&self, path_key: &str) -> Result<Value, MetadatamodelError>;
    /// When `true`, processor errors default the condition to `false`
    /// instead of propagating.
    fn silence_errors(&self) -> bool;
}

/// A condition processor: decides whether `value_found` satisfies
/// `condition` with the parameters in `filter_value` (go's `ConditionTrue`).
pub type ConditionProcessor = fn(
    ctx: &dyn FilterContext,
    path_key: &str,
    condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError>;
