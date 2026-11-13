//! Iter subdomain application use cases (port of go-metadatamodel/iter):
//! recursive walks over the metadata model's fields/groups —
//! [`for_each`] (visit), [`filter`] (remove fields/groups in place), and
//! [`map`] (replace field/group entries in place).
//!
//! All three walk each group's fields in read order (`group:read_order_of_fields`,
//! defaulting to the fields key order) and recurse into groups unless the
//! callback says otherwise. Callbacks receive the field/group's PROPS object
//! (the first element of its entry array).

use dyn_rt_shared::Value;

use super::domain::{FilterDecision, Visit};
use crate::core::application::{
    fields_of, fields_of_mut, is_group, props_of, props_of_mut, read_order_of,
};
use crate::core::domain::{FIELDS_KEY, MetadataModel, props};
use crate::error::MetadatamodelError;

/// Recursively visits every field/group of `group` in read order (go's
/// `iter.ForEach`). The callback receives the field/group's props object.
pub fn for_each(
    group: &MetadataModel,
    callback: &mut dyn FnMut(&Value) -> Visit,
) -> Result<(), MetadatamodelError> {
    let order = read_order_of(group)?;
    let fields = fields_of(group)?;
    let fields_obj = fields
        .as_object()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY}[0] is not an object"),
        })?;

    for key in &order {
        let entry = fields_obj
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, entry)| entry)
            .ok_or_else(|| MetadatamodelError::InvalidModel {
                message: format!("field {key:?} not found in {FIELDS_KEY}[0]"),
            })?;
        let props = props_of(entry)?;
        let visit = callback(props);
        if visit.terminate {
            return Ok(());
        }
        if is_group(props) && !visit.skip_fields {
            for_each(props, callback)?;
        }
    }
    Ok(())
}

/// Recursively removes fields/groups from `group` in place (go's
/// `iter.Filter`): the callback decides per field/group; a removed entry is
/// deleted from the `group:fields[0]` object and from
/// `group:read_order_of_fields` when that prop is present.
pub fn filter(
    group: &mut MetadataModel,
    callback: &mut dyn FnMut(&Value) -> FilterDecision,
) -> Result<(), MetadatamodelError> {
    let order = read_order_of(group)?;

    for key in &order {
        // decide from the current props
        let decision = {
            let fields = fields_of(group)?;
            let entry = fields
                .as_object()
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("{FIELDS_KEY}[0] is not an object"),
                })?
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, entry)| entry)
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("field {key:?} not found in {FIELDS_KEY}[0]"),
                })?;
            callback(props_of(entry)?)
        };

        if !decision.retain {
            remove_field(group, key);
            continue;
        }

        if decision.skip_fields {
            continue;
        }

        // recurse into the group's fields (through the mutable entry)
        let fields = fields_of_mut(group)?;
        let fields_obj =
            fields
                .as_object_mut()
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("{FIELDS_KEY}[0] is not an object"),
                })?;
        let Some(entry) = fields_obj
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, entry)| entry)
        else {
            continue;
        };
        let props = props_of_mut(entry)?;
        if is_group(props) {
            filter(props, callback)?;
        }
    }
    Ok(())
}

/// Removes field/group `key` from the group's fields object and from its
/// `group:read_order_of_fields` array when present.
fn remove_field(group: &mut MetadataModel, key: &str) {
    if let Some(fields) = group.get_mut(FIELDS_KEY) {
        if let Some(array) = fields.as_array_mut() {
            if let Some(obj) = array.first_mut().and_then(Value::as_object_mut) {
                obj.retain(|(k, _)| k != key);
            }
        }
    }
    if let Some(order) = group.get_mut(props::GROUP_READ_ORDER_OF_FIELDS) {
        if let Some(array) = order.as_array_mut() {
            array.retain(|v| v.as_str() != Some(key));
        }
    }
}

/// Recursively replaces field/group entries in place (go's `iter.Map`): the
/// callback receives the current props object and returns the new ENTRY
/// (the array value stored under the field name in `group:fields[0]`) plus
/// whether to skip the nested fields.
pub fn map(
    group: &mut MetadataModel,
    callback: &mut dyn FnMut(&Value) -> (Value, bool),
) -> Result<(), MetadatamodelError> {
    let order = read_order_of(group)?;

    for key in &order {
        // compute the replacement from the current props
        let (new_entry, skip_fields) = {
            let fields = fields_of(group)?;
            let entry = fields
                .as_object()
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("{FIELDS_KEY}[0] is not an object"),
                })?
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, entry)| entry)
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("field {key:?} not found in {FIELDS_KEY}[0]"),
                })?;
            callback(props_of(entry)?)
        };

        // write the new entry back, then recurse into it if it is a group
        let fields = fields_of_mut(group)?;
        let fields_obj =
            fields
                .as_object_mut()
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("{FIELDS_KEY}[0] is not an object"),
                })?;
        let Some(entry) = fields_obj
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, entry)| entry)
        else {
            continue;
        };
        *entry = new_entry;
        if !skip_fields {
            let props = props_of_mut(entry)?;
            if is_group(props) {
                map(props, callback)?;
            }
        }
    }
    Ok(())
}
