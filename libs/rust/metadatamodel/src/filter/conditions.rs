//! Condition processors (go's `IsConditionTrue` / `IsNumberOfEntriesConditionTrue`
//! / `IsTextConditionTrue` / `IsNumberConditionTrue` / `IsTimestampConditionTrue`
//! / `IsDefaultEqualTrue`): decide whether a value found in the source data
//! satisfies one filter condition.

use dyn_rt_json_manipulation::application::are_equal;
use dyn_rt_reflect::{Reflect, ReflectRef};
use dyn_rt_shared::Value;

use super::domain::{
    ConditionProcessor, FilterContext, condition, condition_props, datetime_format, field_type,
};
use crate::error::MetadatamodelError;

/// The default set of condition processors, keyed by condition name (go's
/// `DefaultFilterProcessors`).
pub fn default_filter_processors() -> Vec<(&'static str, ConditionProcessor)> {
    vec![
        (condition::NO_OF_ENTRIES_GREATER_THAN, is_condition_true),
        (condition::NO_OF_ENTRIES_LESS_THAN, is_condition_true),
        (condition::NO_OF_ENTRIES_EQUAL_TO, is_condition_true),
        (condition::GREATER_THAN, is_condition_true),
        (condition::LESS_THAN, is_condition_true),
        (condition::EQUAL_TO, is_condition_true),
        (condition::BEGINS_WITH, is_condition_true),
        (condition::ENDS_WITH, is_condition_true),
        (condition::CONTAINS, is_condition_true),
    ]
}

/// Dispatches one filter condition (go's `IsConditionTrue`): entry-count
/// conditions go to [`number_of_entries_condition_true`]; everything else
/// is dispatched on the condition's `AssumedFieldType` — and when the value
/// found is a sequence, ANY element satisfying the condition counts.
pub fn is_condition_true(
    ctx: &dyn FilterContext,
    path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError> {
    match filter_condition {
        condition::NO_OF_ENTRIES_GREATER_THAN
        | condition::NO_OF_ENTRIES_LESS_THAN
        | condition::NO_OF_ENTRIES_EQUAL_TO => {
            return number_of_entries_condition_true(
                ctx,
                path_key,
                filter_condition,
                value_found,
                filter_value,
            );
        }
        _ => {}
    }

    let assumed = match filter_value.get(condition_props::ASSUMED_FIELD_TYPE) {
        Some(value) => match value.as_str() {
            Some(s) => s.to_string(),
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' is not a string",
                        condition_props::ASSUMED_FIELD_TYPE
                    ),
                );
            }
        },
        None => {
            return error_or_default(
                ctx,
                format!(
                    "filter condition property '{}' not found",
                    condition_props::ASSUMED_FIELD_TYPE
                ),
            );
        }
    };

    if matches!(value_found.reflect_ref(), ReflectRef::Sequence(_)) {
        let sequence = match value_found.reflect_ref() {
            ReflectRef::Sequence(seq) => seq,
            _ => unreachable!(),
        };
        for index in 0..sequence.reflect_length() {
            let element = match sequence.reflect_get(index) {
                Some(element) => element,
                None => continue,
            };
            let condition_true = dispatch_assumed_type(
                ctx,
                path_key,
                filter_condition,
                element,
                filter_value,
                &assumed,
            )?;
            if condition_true {
                return Ok(true);
            }
        }
        return Ok(false);
    }

    dispatch_assumed_type(
        ctx,
        path_key,
        filter_condition,
        value_found,
        filter_value,
        &assumed,
    )
}

/// Routes to the text/number/timestamp/default processor by assumed field
/// type (go's switch in `IsConditionTrue`).
fn dispatch_assumed_type(
    ctx: &dyn FilterContext,
    path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
    assumed: &str,
) -> Result<bool, MetadatamodelError> {
    match assumed {
        field_type::TEXT => {
            text_condition_true(ctx, path_key, filter_condition, value_found, filter_value)
        }
        field_type::NUMBER => {
            number_condition_true(ctx, path_key, filter_condition, value_found, filter_value)
        }
        field_type::TIMESTAMP => {
            timestamp_condition_true(ctx, path_key, filter_condition, value_found, filter_value)
        }
        _ => default_equal_true(ctx, path_key, filter_condition, value_found, filter_value),
    }
}

/// `NoOfEntries*` conditions: compares the length of a sequence value
/// (go's `IsNumberOfEntriesConditionTrue`).
pub fn number_of_entries_condition_true(
    ctx: &dyn FilterContext,
    _path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError> {
    let length = match value_found.reflect_ref() {
        ReflectRef::Sequence(seq) => seq.reflect_length(),
        _ => return Ok(false),
    };

    let values = compare_values_i64(ctx, filter_value)?;
    for value in values {
        let hit = match filter_condition {
            condition::NO_OF_ENTRIES_EQUAL_TO => value == length as i64,
            condition::NO_OF_ENTRIES_GREATER_THAN => value > length as i64,
            condition::NO_OF_ENTRIES_LESS_THAN => value < length as i64,
            other => {
                return error_or_default(ctx, format!("unsupported filter condition '{other}'"));
            }
        };
        if hit {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Text conditions: EqualTo / BeginsWith / EndsWith / Contains against
/// string values, honoring `CaseInsensitive` (go's `IsTextConditionTrue`).
pub fn text_condition_true(
    ctx: &dyn FilterContext,
    _path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError> {
    let case_insensitive = filter_value
        .get(condition_props::CASE_INSENSITIVE)
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut values_to_compare: Vec<String> = Vec::new();
    if let Some(single) = filter_value.get(condition_props::VALUE) {
        match single.as_str() {
            Some(s) => {
                let s = s.to_string();
                values_to_compare.push(if case_insensitive {
                    s.to_lowercase()
                } else {
                    s
                });
            }
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' is not a string",
                        condition_props::VALUE
                    ),
                );
            }
        }
    } else if let Some(multiple) = filter_value.get(condition_props::VALUES) {
        match multiple.as_array() {
            Some(items) => {
                for item in items {
                    match item.as_str() {
                        Some(s) => {
                            let s = s.to_string();
                            values_to_compare.push(if case_insensitive {
                                s.to_lowercase()
                            } else {
                                s
                            });
                        }
                        None => {
                            return error_or_default(
                                ctx,
                                format!(
                                    "filter condition property '{}' entries must be strings",
                                    condition_props::VALUES
                                ),
                            );
                        }
                    }
                }
            }
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' is not an array",
                        condition_props::VALUES
                    ),
                );
            }
        }
    } else {
        return error_or_default(
            ctx,
            format!(
                "filter condition property '{}' not found",
                condition_props::VALUE
            ),
        );
    }

    let value_found_string = match value_found.reflect_scalar_string() {
        Some(s) => s,
        None => return Ok(false),
    };
    let haystack = if case_insensitive {
        value_found_string.to_lowercase()
    } else {
        value_found_string.clone()
    };

    for value in values_to_compare {
        let hit = match filter_condition {
            condition::EQUAL_TO => haystack == value,
            condition::BEGINS_WITH => haystack.starts_with(&value),
            condition::ENDS_WITH => haystack.ends_with(&value),
            condition::CONTAINS => haystack.contains(&value),
            other => {
                return error_or_default(ctx, format!("unsupported filter condition '{other}'"));
            }
        };
        if hit {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Number conditions: EqualTo / GreaterThan / LessThan against numeric
/// values (go's `IsNumberConditionTrue`).
pub fn number_condition_true(
    ctx: &dyn FilterContext,
    _path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError> {
    let values = compare_values_f64(ctx, filter_value)?;

    let value_found_float = match as_f64(value_found) {
        Some(f) => f,
        None => return Ok(false),
    };

    for value in values {
        let hit = match filter_condition {
            condition::EQUAL_TO => value_found_float == value,
            condition::GREATER_THAN => value_found_float > value,
            condition::LESS_THAN => value_found_float < value,
            other => {
                return error_or_default(ctx, format!("unsupported filter condition '{other}'"));
            }
        };
        if hit {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Timestamp conditions: EqualTo / GreaterThan / LessThan on RFC 3339
/// timestamps, compared at the granularity of the condition's
/// `DateTimeFormat` (go's `IsTimestampConditionTrue`).
pub fn timestamp_condition_true(
    ctx: &dyn FilterContext,
    _path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError> {
    let format = match filter_value.get(condition_props::DATE_TIME_FORMAT) {
        Some(value) => match value.as_str() {
            Some(s) => s.to_string(),
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' is not a string",
                        condition_props::DATE_TIME_FORMAT
                    ),
                );
            }
        },
        None => {
            return error_or_default(
                ctx,
                format!(
                    "filter condition property '{}' not found",
                    condition_props::DATE_TIME_FORMAT
                ),
            );
        }
    };

    // collect the comparison values as parsed timestamps
    let mut values_to_compare: Vec<Timestamp> = Vec::new();
    let raw_values: Vec<&Value> = match filter_value.get(condition_props::VALUE) {
        Some(single) => vec![single],
        None => match filter_value.get(condition_props::VALUES) {
            Some(multiple) => match multiple.as_array() {
                Some(items) => items.iter().collect(),
                None => {
                    return error_or_default(
                        ctx,
                        format!(
                            "filter condition property '{}' is not an array",
                            condition_props::VALUES
                        ),
                    );
                }
            },
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' not found",
                        condition_props::VALUE
                    ),
                );
            }
        },
    };
    for raw in raw_values {
        match raw.as_str() {
            Some(s) => match Timestamp::parse_rfc3339(s) {
                Some(ts) => values_to_compare.push(ts),
                None => {
                    return error_or_default(
                        ctx,
                        format!("error parsing filter condition value `{s}` to time"),
                    );
                }
            },
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' must be an RFC 3339 string",
                        condition_props::VALUE
                    ),
                );
            }
        }
    }

    let value_found_string = match value_found.reflect_scalar_string() {
        Some(s) => s,
        None => return Ok(false),
    };
    let value_found_time = match Timestamp::parse_rfc3339(&value_found_string) {
        Some(ts) => ts,
        None => return Ok(false),
    };

    for value in values_to_compare {
        let found_parts = value_found_time.parts_for_format(&format);
        let value_parts = value.parts_for_format(&format);
        let ordering = found_parts.cmp(&value_parts);
        let hit = match filter_condition {
            condition::EQUAL_TO => ordering == std::cmp::Ordering::Equal,
            condition::GREATER_THAN => ordering == std::cmp::Ordering::Greater,
            condition::LESS_THAN => ordering == std::cmp::Ordering::Less,
            other => {
                return error_or_default(ctx, format!("unsupported filter condition '{other}'"));
            }
        };
        if hit {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Default (Any-typed) condition: EqualTo via deep equality (go's
/// `IsDefaultEqualTrue`).
pub fn default_equal_true(
    ctx: &dyn FilterContext,
    _path_key: &str,
    filter_condition: &str,
    value_found: &dyn Reflect,
    filter_value: &Value,
) -> Result<bool, MetadatamodelError> {
    let values: Vec<&Value> = match filter_value.get(condition_props::VALUE) {
        Some(single) => vec![single],
        None => match filter_value.get(condition_props::VALUES) {
            Some(multiple) => match multiple.as_array() {
                Some(items) => items.iter().collect(),
                None => {
                    return error_or_default(
                        ctx,
                        format!(
                            "filter condition property '{}' is not an array",
                            condition_props::VALUES
                        ),
                    );
                }
            },
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' or '{}' not found",
                        condition_props::VALUE,
                        condition_props::VALUES
                    ),
                );
            }
        },
    };

    match filter_condition {
        condition::EQUAL_TO => {
            for value in values {
                if are_equal(value_found, value) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        other => error_or_default(ctx, format!("unsupported filter condition '{other}'")),
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `Value`/​`Values` of the filter condition as i64 (go converts to int).
fn compare_values_i64(
    ctx: &dyn FilterContext,
    filter_value: &Value,
) -> Result<Vec<i64>, MetadatamodelError> {
    let raw: Vec<&Value> = match filter_value.get(condition_props::VALUE) {
        Some(single) => vec![single],
        None => match filter_value.get(condition_props::VALUES) {
            Some(multiple) => match multiple.as_array() {
                Some(items) => items.iter().collect(),
                None => {
                    return error_or_default(
                        ctx,
                        format!(
                            "filter condition property '{}' is not an array",
                            condition_props::VALUES
                        ),
                    );
                }
            },
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' not found",
                        condition_props::VALUE
                    ),
                );
            }
        },
    };
    let mut out = Vec::new();
    for value in raw {
        match value.as_f64() {
            Some(f) => out.push(f as i64),
            None => {
                return error_or_default(ctx, "filter condition value is not a number".to_string());
            }
        }
    }
    Ok(out)
}

/// `Value`/​`Values` of the filter condition as f64 (numeric strings are
/// parsed too, mirroring go's conversion to float64).
fn compare_values_f64(
    ctx: &dyn FilterContext,
    filter_value: &Value,
) -> Result<Vec<f64>, MetadatamodelError> {
    let raw: Vec<&Value> = match filter_value.get(condition_props::VALUE) {
        Some(single) => vec![single],
        None => match filter_value.get(condition_props::VALUES) {
            Some(multiple) => match multiple.as_array() {
                Some(items) => items.iter().collect(),
                None => {
                    return error_or_default(
                        ctx,
                        format!(
                            "filter condition property '{}' is not an array",
                            condition_props::VALUES
                        ),
                    );
                }
            },
            None => {
                return error_or_default(
                    ctx,
                    format!(
                        "filter condition property '{}' not found",
                        condition_props::VALUE
                    ),
                );
            }
        },
    };
    let mut out = Vec::new();
    for value in raw {
        match as_f64_str(value) {
            Some(f) => out.push(f),
            None => {
                return error_or_default(ctx, "filter condition value is not a number".to_string());
            }
        }
    }
    Ok(out)
}

/// Numeric view of a reflected value or a `Value` (numbers as-is, numeric
/// strings parsed).
fn as_f64(value: &dyn Reflect) -> Option<f64> {
    if let Some(f) = value.reflect_scalar_f64() {
        return Some(f);
    }
    if let Some(i) = value.reflect_scalar_i64() {
        return Some(i as f64);
    }
    if let Some(s) = value.reflect_scalar_string() {
        return s.trim().parse::<f64>().ok();
    }
    None
}

fn as_f64_str(value: &Value) -> Option<f64> {
    match value.as_f64() {
        Some(f) => Some(f),
        None => value.as_str().and_then(|s| s.trim().parse::<f64>().ok()),
    }
}

/// When the context silences errors, the result defaults (bool -> `false`,
/// Vec -> empty) instead of failing (go's `returnErrorOrFalse`).
fn error_or_default<T: Default>(
    ctx: &dyn FilterContext,
    message: String,
) -> Result<T, MetadatamodelError> {
    if ctx.silence_errors() {
        return Ok(T::default());
    }
    Err(MetadatamodelError::Filter { message })
}

// ---------------------------------------------------------------------------
// timestamps (std-only, explicit RFC 3339 parsing — no external date crate
// in core per ADR 0002)
// ---------------------------------------------------------------------------

/// A parsed RFC 3339 timestamp: civil date-time parts plus the UTC offset
/// in minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub offset_minutes: i32,
}

impl Timestamp {
    /// Parses `YYYY-MM-DDTHH:MM:SS[.fraction][Z|±HH:MM]` explicitly.
    pub fn parse_rfc3339(input: &str) -> Option<Self> {
        let bytes = input.as_bytes();
        if bytes.len() < 20 {
            return None;
        }
        // YYYY-MM-DD
        let year = parse_digits(&input[0..4])?;
        if input.as_bytes().get(4) != Some(&b'-') {
            return None;
        }
        let month = parse_digits(&input[5..7])? as u32;
        if input.as_bytes().get(7) != Some(&b'-') {
            return None;
        }
        let day = parse_digits(&input[8..10])? as u32;
        if input.as_bytes().get(10) != Some(&b'T') && input.as_bytes().get(10) != Some(&b' ') {
            return None;
        }
        let hour = parse_digits(&input[11..13])? as u32;
        if input.as_bytes().get(13) != Some(&b':') {
            return None;
        }
        let minute = parse_digits(&input[14..16])? as u32;
        if input.as_bytes().get(16) != Some(&b':') {
            return None;
        }
        let second = parse_digits(&input[17..19])? as u32;

        // optional fractional seconds (skipped — second granularity is
        // enough for the supported datetime formats)
        let mut rest = &input[19..];
        if rest.starts_with('.') {
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest = &rest[end..];
        }

        // zone: Z or ±HH:MM
        let offset_minutes = if rest == "Z" || rest.is_empty() {
            0
        } else if rest.starts_with('+') || rest.starts_with('-') {
            let sign = if rest.starts_with('-') { -1 } else { 1 };
            let offset = &rest[1..];
            if offset.len() < 5 || offset.as_bytes().get(2) != Some(&b':') {
                return None;
            }
            let oh = parse_digits(&offset[0..2])? as i32;
            let om = parse_digits(&offset[3..5])? as i32;
            sign * (oh * 60 + om)
        } else {
            return None;
        };

        Some(Timestamp {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes,
        })
    }

    /// The comparison parts at the granularity of the datetime format
    /// (go compares year, then month, then day, ... per format).
    pub fn parts_for_format(&self, format: &str) -> Vec<i64> {
        match format {
            datetime_format::YYYY => vec![self.year],
            datetime_format::YYYYMM => vec![self.year, self.month as i64],
            datetime_format::YYYYMMDD => {
                vec![self.year, self.month as i64, self.day as i64]
            }
            datetime_format::YYYYMMDDHHMM => vec![
                self.year,
                self.month as i64,
                self.day as i64,
                self.hour as i64,
                self.minute as i64,
            ],
            datetime_format::MM => vec![self.month as i64],
            datetime_format::HHMM => vec![self.hour as i64, self.minute as i64],
            // unknown format: full timestamp
            _ => vec![
                self.year,
                self.month as i64,
                self.day as i64,
                self.hour as i64,
                self.minute as i64,
                self.second as i64,
            ],
        }
    }
}

/// Parses exactly `s.len()` ASCII digits as i64.
fn parse_digits(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<i64>().ok()
}
