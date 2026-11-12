//! Schema domain model: a JSON-like schema tree describing the expected
//! structure of data, keyed off the reflect module's kinds.
//!
//! Port of go-json's `DynamicSchemaNode` / `DynamicSchema`. The Go struct
//! carried every child-schema slot as an optional field; here the shape is
//! a closed enum (`SchemaShape`) so each variant carries exactly the child
//! schemas it uses. `Schema::Set` (go's `DynamicSchema`) models a value that
//! may take one of several shapes (polymorphism), tried in order with an
//! optional default key.

use std::any::TypeId;
use std::sync::Arc;

use dyn_rt_reflect::{Reflect, ReflectKind, TypeInfo, type_info};
use dyn_rt_shared::Value;

use crate::error::SchemaError;

/// One step in a schema path: root, named key, or positional index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaPathSegment {
    Root,
    Key(String),
    Index(usize),
}

/// A schema path: the steps from the schema root down to a node.
pub type SchemaPath = Vec<SchemaPathSegment>;

impl SchemaPathSegment {
    /// Renders the segment in JSONPath form (`$`, `.key` / `['key']`, `[i]`).
    pub fn render(&self, out: &mut String) {
        match self {
            SchemaPathSegment::Root => out.push('$'),
            SchemaPathSegment::Key(key) => {
                if key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                    && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    out.push('.');
                    out.push_str(key);
                } else {
                    out.push_str("['");
                    out.push_str(key);
                    out.push_str("']");
                }
            }
            SchemaPathSegment::Index(index) => {
                out.push('[');
                out.push_str(&index.to_string());
                out.push(']');
            }
        }
    }
}

/// Renders a schema path in JSONPath form, e.g. `$.Address.Street`.
pub fn render_path(path: &[SchemaPathSegment]) -> String {
    let mut out = String::new();
    for segment in path {
        segment.render(&mut out);
    }
    out
}

/// Custom per-node validation logic (port of go's `Validator`).
pub trait SchemaValidator: Send + Sync {
    fn validate(
        &self,
        data: &dyn Reflect,
        node: &SchemaNode,
        path: &[SchemaPathSegment],
    ) -> Result<(), SchemaError>;
}

/// Custom per-node conversion logic (port of go's `Converter`).
pub trait SchemaConverter: Send + Sync {
    fn convert(
        &self,
        data: &dyn Reflect,
        node: &SchemaNode,
        path: &[SchemaPathSegment],
    ) -> Result<Box<dyn Reflect>, SchemaError>;
}

/// A schema: either a single node or a set of alternative nodes.
#[derive(Debug, Clone)]
pub enum Schema {
    Node(SchemaNode),
    Set(SchemaSet),
}

/// A collection of alternative schema nodes (go's `DynamicSchema`), e.g. for
/// polymorphic values. `default_key` is tried first during validation and
/// path resolution.
#[derive(Debug, Clone)]
pub struct SchemaSet {
    pub default_key: Option<String>,
    pub nodes: Vec<(String, SchemaNode)>,
}

/// A single node in the schema tree (go's `DynamicSchemaNode`).
#[derive(Clone)]
pub struct SchemaNode {
    /// The expected concrete type of the data (identity is `type_id`;
    /// `type_name` is a display label only — see reflect's `Reflect` docs).
    pub type_info: TypeInfo,
    /// The structural shape and its child schemas.
    pub shape: SchemaShape,
    /// Whether the data may be nil (e.g. `Option::None`).
    pub nilable: bool,
    /// Optional default value constructor for new initializations.
    pub default: Option<fn() -> Box<dyn Reflect>>,
    /// When true, every child schema entry must be matched by the data.
    pub children_must_be_valid: bool,
    /// Key schema for this node when it appears as a map entry value.
    pub entry_key_schema: Option<Box<Schema>>,
    /// Custom validator for this specific node.
    pub validator: Option<Arc<dyn SchemaValidator>>,
    /// Custom converter for this specific node.
    pub converter: Option<Arc<dyn SchemaConverter>>,
}

/// The structural shape of a schema node. Unlike go's flat struct with
/// optional child-schema fields, each variant carries exactly the child
/// schemas its shape uses.
#[derive(Debug, Clone)]
pub enum SchemaShape {
    /// A final scalar value (bool, numbers, string, char, ...).
    Scalar,
    /// Accepts anything (go's `reflect.Interface`).
    Any,
    /// A struct: named fields, each with its own schema.
    Struct { fields: Vec<(String, Schema)> },
    /// A map: optional per-key entry schemas, plus key/value schemas for all
    /// other entries.
    Map {
        entries: Vec<(String, Schema)>,
        key: Option<Box<Schema>>,
        value: Option<Box<Schema>>,
    },
    /// A sequence: optional per-index item schemas, plus an element schema
    /// for all other positions.
    Sequence {
        elements: Option<Box<Schema>>,
        items: Vec<(usize, Schema)>,
    },
    /// A pointer-like wrapper (Box/Rc/Arc/...): schema for the inner value.
    Indirection { inner: Box<Schema> },
    /// An enum: variant name -> optional payload schema.
    Enum {
        variants: Vec<(String, Option<Box<Schema>>)>,
    },
}

impl std::fmt::Debug for SchemaNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaNode")
            .field("type_info", &self.type_info)
            .field("shape", &self.shape)
            .field("nilable", &self.nilable)
            .field("default", &self.default.map(|_| "<fn>"))
            .field("children_must_be_valid", &self.children_must_be_valid)
            .field("entry_key_schema", &self.entry_key_schema)
            .field("validator", &self.validator.as_ref().map(|_| "<custom>"))
            .field("converter", &self.converter.as_ref().map(|_| "<custom>"))
            .finish()
    }
}

impl SchemaNode {
    /// The reflect kind this node expects data to have.
    pub fn kind(&self) -> ReflectKind {
        match &self.shape {
            SchemaShape::Scalar => ReflectKind::Scalar,
            // `Any` accepts every kind; Unknown is the base fallback kind.
            SchemaShape::Any => ReflectKind::Unknown,
            SchemaShape::Struct { .. } => ReflectKind::Struct,
            SchemaShape::Map { .. } => ReflectKind::Map,
            SchemaShape::Sequence { .. } => ReflectKind::Sequence,
            SchemaShape::Indirection { .. } => ReflectKind::Indirection,
            SchemaShape::Enum { .. } => ReflectKind::Enum,
        }
    }

    fn base<T: Reflect>(shape: SchemaShape) -> Self {
        SchemaNode {
            type_info: TypeInfo::of::<T>(),
            shape,
            nilable: false,
            default: None,
            children_must_be_valid: false,
            entry_key_schema: None,
            validator: None,
            converter: None,
        }
    }

    /// A scalar node for the concrete type `T`.
    pub fn scalar<T: Reflect>() -> Self {
        Self::base::<T>(SchemaShape::Scalar)
    }

    /// A scalar node for a `shared::Value` scalar variant (e.g. `"string"`,
    /// `"number"`, `"boolean"`, `"null"`), matched by variant name.
    pub fn json_scalar(variant_name: &'static str) -> Self {
        SchemaNode {
            type_info: TypeInfo {
                type_name: variant_name,
                type_id: TypeId::of::<Value>(),
                kind: ReflectKind::Scalar,
                new_instance: None,
            },
            shape: SchemaShape::Scalar,
            nilable: false,
            default: None,
            children_must_be_valid: false,
            entry_key_schema: None,
            validator: None,
            converter: None,
        }
    }

    /// A node accepting any value.
    pub fn any() -> Self {
        Self::base::<Value>(SchemaShape::Any)
    }

    pub fn struct_node<T: Reflect>(fields: Vec<(&str, Schema)>) -> Self {
        Self::base::<T>(SchemaShape::Struct {
            fields: fields
                .into_iter()
                .map(|(name, schema)| (name.to_string(), schema))
                .collect(),
        })
    }

    pub fn map_node<T: Reflect>(key: Schema, value: Schema) -> Self {
        Self::base::<T>(SchemaShape::Map {
            entries: Vec::new(),
            key: Some(Box::new(key)),
            value: Some(Box::new(value)),
        })
    }

    pub fn map_node_with_entries<T: Reflect>(
        entries: Vec<(&str, Schema)>,
        key: Option<Schema>,
        value: Option<Schema>,
    ) -> Self {
        Self::base::<T>(SchemaShape::Map {
            entries: entries
                .into_iter()
                .map(|(name, schema)| (name.to_string(), schema))
                .collect(),
            key: key.map(Box::new),
            value: value.map(Box::new),
        })
    }

    pub fn sequence_node<T: Reflect>(elements: Schema) -> Self {
        Self::base::<T>(SchemaShape::Sequence {
            elements: Some(Box::new(elements)),
            items: Vec::new(),
        })
    }

    pub fn sequence_node_with_items<T: Reflect>(
        elements: Schema,
        items: Vec<(usize, Schema)>,
    ) -> Self {
        Self::base::<T>(SchemaShape::Sequence {
            elements: Some(Box::new(elements)),
            items,
        })
    }

    pub fn indirection_node<T: Reflect>(inner: Schema) -> Self {
        Self::base::<T>(SchemaShape::Indirection {
            inner: Box::new(inner),
        })
    }

    pub fn enum_node<T: Reflect>(variants: Vec<(&str, Option<Schema>)>) -> Self {
        Self::base::<T>(SchemaShape::Enum {
            variants: variants
                .into_iter()
                .map(|(name, schema)| (name.to_string(), schema.map(Box::new)))
                .collect(),
        })
    }

    pub fn nilable(mut self) -> Self {
        self.nilable = true;
        self
    }

    pub fn children_must_be_valid(mut self) -> Self {
        self.children_must_be_valid = true;
        self
    }

    pub fn with_default(mut self, default: fn() -> Box<dyn Reflect>) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_validator(mut self, validator: Arc<dyn SchemaValidator>) -> Self {
        self.validator = Some(validator);
        self
    }

    pub fn with_converter(mut self, converter: Arc<dyn SchemaConverter>) -> Self {
        self.converter = Some(converter);
        self
    }
}

impl SchemaSet {
    pub fn new(default_key: Option<&str>, nodes: Vec<(&str, SchemaNode)>) -> Self {
        SchemaSet {
            default_key: default_key.map(|key| key.to_string()),
            nodes: nodes
                .into_iter()
                .map(|(key, node)| (key.to_string(), node))
                .collect(),
        }
    }
}

/// Type info for a concrete type, for schema nodes over typed data.
pub fn schema_type_info<T: Reflect>() -> TypeInfo {
    type_info::<T>().clone()
}
