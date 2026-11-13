//! Core domain: the metadata model as a JSON-like structure and its
//! vocabulary.
//!
//! The metadata model is NOT a concrete struct: it is a `shared::Value`
//! tree (objects/arrays/scalars), so it is extensible (unknown properties
//! are carried along untouched) and can itself be traversed, validated and
//! converted with the json modules. A *group* is an object; its fields live
//! under the structural key [`FIELDS_KEY`], which is ALWAYS an array whose
//! first element is the object mapping field-name → field/group. Field
//! props are likewise always an array (one props object), so a field's
//! `group:path_key` template carries `[*]` all the way to the end, e.g.
//! `$['group:fields'][*].Bio[*]`. The same template fetches the field/group
//! props (schema path, with indices substituted) and is post-processed into
//! the actual path to data (see `application`).

use dyn_rt_shared::Value;

/// Structural key holding a group's fields (always an array in the model).
/// Namespaced like the other keys, so templates use the bracket form:
/// `$['group:fields'][*].Bio[*]`.
pub const FIELDS_KEY: &str = "group:fields";

/// Property keys, namespaced `<section>:<key>` (refinement of go's flat
/// `FieldGroup*`/`Field*`/`Group*`/`Database*` names). Unknown sections are
/// simply carried along — the model is extensible by construction.
pub mod props {
    // group section
    pub const GROUP_PATH_KEY: &str = "group:path_key";
    pub const GROUP_NAME: &str = "group:name";
    pub const GROUP_DESCRIPTION: &str = "group:description";
    pub const GROUP_MAX_ENTRIES: &str = "group:max_entries";
    pub const GROUP_IS_PRIMARY_KEY: &str = "group:is_primary_key";
    pub const GROUP_READ_ORDER_OF_FIELDS: &str = "group:read_order_of_fields";
    pub const GROUP_TYPE_ANY: &str = "group:type_any";
    pub const GROUP_CARDINALITY: &str = "group:cardinality";

    // field section
    pub const FIELD_DATA_TYPE: &str = "field:data_type";
    pub const FIELD_UI: &str = "field:ui";
    pub const FIELD_CARDINALITY: &str = "field:cardinality";
    pub const FIELD_PLACEHOLDER: &str = "field:placeholder";
    pub const FIELD_DEFAULT_VALUE: &str = "field:default_value";
    pub const FIELD_DATETIME_FORMAT: &str = "field:datetime_format";
    pub const FIELD_SELECT_OPTIONS: &str = "field:select_options";
    pub const FIELD_CHECKBOX_VALUE_IF_TRUE: &str = "field:checkbox_value_if_true";
    pub const FIELD_CHECKBOX_VALUE_IF_FALSE: &str = "field:checkbox_value_if_false";
    pub const FIELD_CHECKBOX_VALUES_USE_IN_VIEW: &str = "field:checkbox_values_use_in_view";
    pub const FIELD_CHECKBOX_VALUES_USE_IN_STORAGE: &str = "field:checkbox_values_use_in_storage";

    // view section
    pub const VIEW_TABLE_LOCK_COLUMN: &str = "view:table_lock_column";
    pub const VIEW_VALUES_IN_SEPARATE_COLUMNS: &str = "view:values_in_separate_columns";
    pub const VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS: &str = "view:max_values_in_separate_columns";
    pub const VIEW_SEPARATE_COLUMNS_HEADER_FORMAT: &str = "view:separate_columns_header_format";
    pub const VIEW_SEPARATE_COLUMNS_HEADER_INDEX: &str = "view:separate_columns_header_index";
    pub const VIEW_MULTIPLE_VALUES_JOIN_SYMBOL: &str = "view:multiple_values_join_symbol";
    pub const VIEW_DISABLE: &str = "view:disable";
    pub const VIEW_TABLE_IN_2D: &str = "view:table_in_2d";
    pub const VIEW_COLUMN_POSITION: &str = "view:column_position";
    pub const VIEW_POSITION_BEFORE: &str = "view:position_before";

    // input/edit section
    pub const INPUT_DISABLE: &str = "input:disable";
    pub const INPUT_PLACEHOLDER: &str = "input:placeholder";
    pub const EDIT_DISABLE_PROPERTIES: &str = "edit:disable_properties";

    // query section
    pub const QUERY_DISABLE_CONDITIONS_EDIT: &str = "query:disable_conditions_edit";
    pub const QUERY_ADD_FULL_TEXT_SEARCH_BOX: &str = "query:add_full_text_search_box";

    // extract section
    pub const EXTRACT_AS_SINGLE_FIELD: &str = "extract:as_single_field";

    // database section
    pub const DB_FIELD_ADD_TO_FULL_TEXT_SEARCH_INDEX: &str =
        "db:field_add_to_full_text_search_index";
    pub const DB_SKIP_DATA_EXTRACTION: &str = "db:skip_data_extraction";
    pub const DB_TABLE_COLLECTION_UID: &str = "db:table_collection_uid";
    pub const DB_TABLE_COLLECTION_NAME: &str = "db:table_collection_name";
    pub const DB_TABLE_NAME: &str = "db:table_name";
    pub const DB_COLUMN_NAME: &str = "db:column_name";
    pub const DB_FIELD_VALUE_TYPE: &str = "db:field_value_type";
}

/// Whether a field/group's VALUES are a single value or a list.
///
/// Default is [`Cardinality::Array`] (go compatibility). Choosing
/// [`Cardinality::Single`] is an explicit, committed choice: every value of
/// the field/group must be single — if the actual data contradicts the
/// declaration, that is a [`crate::error::MetadatamodelError::CardinalityViolation`],
/// never a dynamic exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cardinality {
    /// One single value (object/scalar) per field/group entry.
    Single,
    /// Every value is a list (array). This is the default.
    #[default]
    Array,
}

impl Cardinality {
    /// Parses the property value (`"single"`/`"object"` → Single,
    /// `"array"`/`"list"` → Array).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "single" | "object" => Some(Cardinality::Single),
            "array" | "list" => Some(Cardinality::Array),
            _ => None,
        }
    }

    /// The canonical property value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Cardinality::Single => "single",
            Cardinality::Array => "array",
        }
    }
}

/// The metadata model document: a JSON-like [`Value`] tree. Not a concrete
/// struct — see the module docs.
pub type MetadataModel = Value;
