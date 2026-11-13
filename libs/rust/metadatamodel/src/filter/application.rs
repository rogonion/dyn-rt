//! Filter subdomain application use case (port of go-metadatamodel/filter's
//! `DataFilter`): evaluate a query condition against every entry of a root
//! sequence in the source data and return the indexes that DID NOT pass.
//!
//! Query condition shapes (go's doc.go example):
//! - `{"Type": "LogicalOperator", "LogicalOperator": "And"|"Or", "Negate": bool,
//!     "Value": [<query condition>, ...]}`
//! - `{"Type": "FieldGroup", "Negate": bool, "Value": {"<schema path key>": {"<condition>": {...}}}}`

use dyn_rt_json_manipulation::application::{for_each, get};
use dyn_rt_reflect::{Reflect, ReflectKind};
use dyn_rt_shared::Value;

use super::conditions::default_filter_processors;
use super::domain::{ConditionProcessor, FilterContext, LogicalOperator, query};
use crate::core::application::{data_path_keep_wildcards, parse_template};
use crate::core::domain::MetadataModel;
use crate::error::MetadatamodelError;

/// Filters source data whose structure is defined by a metadata model (go's
/// `DataFilter`).
pub struct DataFilter<'a> {
    /// The source data to filter (the root value at `root_json_path_to_value`
    /// must be a sequence).
    pub source: &'a dyn Reflect,
    /// The metadata model describing the source data.
    pub metadata_model: &'a MetadataModel,
    /// Condition processors keyed by condition name.
    pub processors: Vec<(&'static str, ConditionProcessor)>,
    /// When `true`, errors default the current condition to `false`.
    pub silence_errors: bool,
    /// Sub-set of the metadata model acting as root context (default `$`).
    pub root_json_path_key: String,
    /// Path to the data in `source` acting as root context (default `$`).
    pub root_json_path_to_value: String,
}

impl<'a> DataFilter<'a> {
    pub fn new(source: &'a dyn Reflect, metadata_model: &'a MetadataModel) -> Self {
        DataFilter {
            source,
            metadata_model,
            processors: default_filter_processors(),
            silence_errors: false,
            root_json_path_key: "$".to_string(),
            root_json_path_to_value: "$".to_string(),
        }
    }

    /// Runs `query_condition` against every entry of the root sequence and
    /// returns the indexes that did NOT pass the filter (go's
    /// `DataFilter.Filter`).
    pub fn filter(
        &mut self,
        query_condition: &Value,
        root_json_path_key: &str,
        root_json_path_to_value: &str,
    ) -> Result<Vec<usize>, MetadatamodelError> {
        if !root_json_path_key.is_empty() {
            self.root_json_path_key = root_json_path_key.to_string();
        }
        if !root_json_path_to_value.is_empty() {
            self.root_json_path_to_value = root_json_path_to_value.to_string();
        }

        let root_segments = parse_template(&self.root_json_path_to_value)?;
        let root_values =
            get(self.source, &root_segments).map_err(|err| MetadatamodelError::Manipulation {
                message: err.to_string(),
            })?;
        let root_value = root_values
            .first()
            .ok_or_else(|| MetadatamodelError::Filter {
                message: "get root value yielded 0 results".to_string(),
            })?;
        if !matches!(root_value.reflect_ref().kind(), ReflectKind::Sequence) {
            return Err(MetadatamodelError::Filter {
                message: "root value should be a sequence".to_string(),
            });
        }

        // iterate the root sequence's entries: `<root>[*]`
        let elements_path = format!("{}[*]", self.root_json_path_to_value);
        let elements_segments = parse_template(&elements_path)?;
        let elements = get(self.source, &elements_segments).map_err(|err| {
            MetadatamodelError::Manipulation {
                message: err.to_string(),
            }
        })?;

        let mut exclude_indexes: Vec<usize> = Vec::new();
        let query_is_empty = query_condition.as_object().is_none_or(|obj| obj.is_empty());
        for (index, element) in elements.iter().enumerate() {
            if query_is_empty {
                continue; // empty query keeps everything
            }
            let ok = self.is_query_condition_true(query_condition, *element)?;
            if !ok {
                exclude_indexes.push(index);
            }
        }
        Ok(exclude_indexes)
    }

    /// Evaluates one query condition against the current value (go's
    /// `isQueryConditionTrue`).
    fn is_query_condition_true(
        &self,
        query_condition: &Value,
        current_value: &dyn Reflect,
    ) -> Result<bool, MetadatamodelError> {
        let condition_type = match query_condition.get(query::TYPE).and_then(Value::as_str) {
            Some(t) => t,
            None => {
                return self.error_or_false(format!("key '{}' is not valid", query::TYPE));
            }
        };
        match condition_type {
            query::TYPE_LOGICAL_OPERATOR => {
                self.is_logical_operator_true(query_condition, current_value)
            }
            query::TYPE_FIELD_GROUP => self.is_field_group_true(query_condition, current_value),
            other => self.error_or_false(format!("unknown query condition type: {other}")),
        }
    }

    /// `LogicalOperator` condition: `Value` is an array of sub-conditions
    /// combined with And/Or, then flipped when `Negate` (go's
    /// `isRecursiveLogicalOperatorTrue`).
    fn is_logical_operator_true(
        &self,
        query_condition: &Value,
        current_value: &dyn Reflect,
    ) -> Result<bool, MetadatamodelError> {
        let negate = negate_of(query_condition);
        let operator = logical_operator_of(self, query_condition)?;
        let conditions = match query_condition.get(query::VALUE).and_then(Value::as_array) {
            Some(items) => items,
            None => {
                return self.error_or_false(format!("key '{}' is not valid", query::VALUE));
            }
        };

        let mut results: Vec<bool> = Vec::new();
        for condition in conditions {
            if condition.as_object().is_none() {
                return self.error_or_false("condition is not an object".to_string());
            }
            let condition_true = self.is_query_condition_true(condition, current_value)?;
            results.push(condition_true);
        }

        let combined = match operator {
            LogicalOperator::Or => results.iter().any(|r| *r),
            LogicalOperator::And => results.iter().all(|r| *r),
        };
        Ok(if negate { !combined } else { combined })
    }

    /// `FieldGroup` condition: `Value` is an object mapping schema path keys
    /// to per-field conditions, combined with And/Or, then flipped when
    /// `Negate` (go's `isRecursiveFieldGroupTrue`).
    fn is_field_group_true(
        &self,
        query_condition: &Value,
        current_value: &dyn Reflect,
    ) -> Result<bool, MetadatamodelError> {
        let negate = negate_of(query_condition);
        let operator = logical_operator_of(self, query_condition)?;
        let conditions = match query_condition.get(query::VALUE).and_then(Value::as_object) {
            Some(obj) => obj,
            None => {
                return self.error_or_false(format!("key '{}' is not valid", query::VALUE));
            }
        };

        let mut results: Vec<bool> = Vec::new();
        for (path_key, condition) in conditions {
            if condition.as_object().is_none() {
                return self.error_or_false("condition is not an object".to_string());
            }
            let condition_true =
                self.is_field_group_condition_true(path_key, condition, current_value)?;
            results.push(condition_true);
        }

        let combined = match operator {
            LogicalOperator::Or => results.iter().any(|r| *r),
            LogicalOperator::And => results.iter().all(|r| *r),
        };
        Ok(if negate { !combined } else { combined })
    }

    /// One field's conditions: resolves the schema path key to a data path
    /// (wildcards kept), iterates every matched value, and ANDs the
    /// conditions; OR over the matched values (go's
    /// `isFieldGroupConditionTrue`).
    fn is_field_group_condition_true(
        &self,
        path_key: &str,
        query_condition: &Value,
        current_value: &dyn Reflect,
    ) -> Result<bool, MetadatamodelError> {
        if query_condition.as_object().is_none_or(|obj| obj.is_empty()) {
            return self.error_or_false("query condition is empty".to_string());
        }

        // root-relative schema path key -> path to data in the source value
        let current_key = path_key.replacen(&self.root_json_path_key, "$", 1);
        let to_value = data_path_keep_wildcards(&current_key)?;
        let segments = parse_template(&to_value)?;

        let mut or_condition_true = false;
        let mut loop_error: Option<MetadatamodelError> = None;
        let conditions = query_condition
            .as_object()
            .ok_or_else(|| MetadatamodelError::Filter {
                message: "query condition is not an object".to_string(),
            })?;

        let result = for_each(current_value, &segments, &mut |value| {
            // AND over this field's conditions
            for (condition_key, condition_data) in conditions {
                let Some(processor) = self.processor(condition_key) else {
                    if self.silence_errors {
                        continue;
                    }
                    loop_error = Some(MetadatamodelError::Filter {
                        message: format!("no processor for filter condition '{condition_key}'"),
                    });
                    return false;
                };
                match processor(self, path_key, condition_key, value, condition_data) {
                    Ok(true) => {}
                    Ok(false) => return true, // this value fails the AND: try the next value
                    Err(err) => {
                        if self.silence_errors {
                            continue;
                        }
                        loop_error = Some(err);
                        return false;
                    }
                }
            }
            // all conditions true for this value
            or_condition_true = true;
            false // stop iterating
        });
        if let Err(err) = result {
            return self.error_or_false(err.to_string());
        }
        if let Some(err) = loop_error {
            return self.error_or_false_err(err);
        }
        Ok(or_condition_true)
    }

    fn processor(&self, condition_key: &str) -> Option<ConditionProcessor> {
        self.processors
            .iter()
            .find(|(name, _)| *name == condition_key)
            .map(|(_, p)| *p)
    }

    /// When `silence_errors` is set, errors become `false` (go's
    /// `returnErrorOrFalse`).
    fn error_or_false(&self, message: String) -> Result<bool, MetadatamodelError> {
        if self.silence_errors {
            return Ok(false);
        }
        Err(MetadatamodelError::Filter { message })
    }

    fn error_or_false_err(&self, err: MetadatamodelError) -> Result<bool, MetadatamodelError> {
        if self.silence_errors {
            return Ok(false);
        }
        Err(err)
    }
}

impl FilterContext for DataFilter<'_> {
    fn get_field_group_by_path_key(&self, path_key: &str) -> Result<Value, MetadatamodelError> {
        let segments = parse_template(path_key)?;
        let values = get(self.metadata_model, &segments).map_err(|err| {
            MetadatamodelError::Manipulation {
                message: err.to_string(),
            }
        })?;
        let first = values.first().ok_or_else(|| MetadatamodelError::Filter {
            message: "get field/group failed".to_string(),
        })?;
        match first.reflect_as_any().downcast_ref::<Value>() {
            Some(value) => Ok(value.clone()),
            None => Err(MetadatamodelError::Filter {
                message: "value found is not a model Value".to_string(),
            }),
        }
    }

    fn silence_errors(&self) -> bool {
        self.silence_errors
    }
}

/// The `Negate` flag of a query condition (default false).
fn negate_of(query_condition: &Value) -> bool {
    query_condition
        .get(query::NEGATE)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The `LogicalOperator` of a query condition (default And; must be valid).
fn logical_operator_of(
    ctx: &DataFilter,
    query_condition: &Value,
) -> Result<LogicalOperator, MetadatamodelError> {
    let raw = query_condition
        .get(query::LOGICAL_OPERATOR)
        .and_then(Value::as_str)
        .unwrap_or(query::AND);
    match LogicalOperator::parse(raw) {
        Some(op) => Ok(op),
        None => {
            if ctx.silence_errors {
                Ok(LogicalOperator::And)
            } else {
                Err(MetadatamodelError::Filter {
                    message: format!("invalid logical operator '{raw}'"),
                })
            }
        }
    }
}
