//! `DynObject`: the stateful entrypoint wrapper over the pure
//! `get` / `set` / `delete` / `for_each` use cases (the Rust analogue of go's
//! `object.Object` / js's `JSObject`, named `DynObject` because `Object`
//! clashes with TypeScript's global `Object`). It wraps a source value —
//! borrowed or owned — plus an optional schema, and accepts a JSONPath in any
//! representation (text, parsed segment(s)), so callers do not have to parse
//! paths themselves. It deliberately holds no result state (no last-error /
//! result counters); a future access-index module can be held here as an
//! optional field for direct-access execution.

use dyn_rt_path::{
    RecursiveDescentSegment, RecursiveDescentSegments, RegexPathTokenizer, parse_path,
};
use dyn_rt_reflect::Reflect;
use dyn_rt_schema::{Schema, SchemaNode};

use super::{delete, for_each, get, set};
use crate::error::ManipulationError;

/// How a [`DynObject`] holds its source value: shared (read-only), borrowed
/// mutable, or owned by the wrapper itself (for cases where there is no
/// separate source owner to deal with).
pub enum Source<'a> {
    /// Shared (read-only) source — `set`/`delete` on it fail.
    Shared(&'a dyn Reflect),
    /// Borrowed mutable source.
    Borrowed(&'a mut dyn Reflect),
    /// Owned source; hand it back with [`DynObject::into_source`].
    Owned(Box<dyn Reflect>),
}

impl Source<'_> {
    /// The source as a shared reference.
    pub fn as_reflect(&self) -> &dyn Reflect {
        match self {
            Source::Shared(r) => *r,
            Source::Borrowed(r) => &**r,
            Source::Owned(b) => &**b,
        }
    }

    /// The source as a mutable reference, or `None` for a shared (read-only)
    /// source.
    pub fn as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        match self {
            Source::Shared(_) => None,
            Source::Borrowed(r) => Some(&mut **r),
            Source::Owned(b) => Some(&mut **b),
        }
    }

    /// Consume into an owned box (a borrowed/shared source is deep-cloned via
    /// `reflect_clone`).
    pub fn into_owned(self) -> Box<dyn Reflect> {
        match self {
            Source::Shared(r) => r.reflect_clone(),
            Source::Borrowed(r) => r.reflect_clone(),
            Source::Owned(b) => b,
        }
    }
}

/// A JSONPath in any accepted representation: text (parsed on the spot via
/// path's tokenizer), a single parsed segment, or parsed segments.
pub enum PathInput<'a> {
    /// A JSONPath string, e.g. `$.a[0].b`.
    Text(&'a str),
    /// One parsed recursive-descent segment.
    Segment(&'a RecursiveDescentSegment),
    /// Parsed recursive-descent segments.
    Segments(&'a RecursiveDescentSegments),
}

impl<'a> From<&'a str> for PathInput<'a> {
    fn from(value: &'a str) -> Self {
        PathInput::Text(value)
    }
}

impl<'a> From<&'a RecursiveDescentSegment> for PathInput<'a> {
    fn from(value: &'a RecursiveDescentSegment) -> Self {
        PathInput::Segment(value)
    }
}

impl<'a> From<&'a RecursiveDescentSegments> for PathInput<'a> {
    fn from(value: &'a RecursiveDescentSegments) -> Self {
        PathInput::Segments(value)
    }
}

impl PathInput<'_> {
    /// Resolve to parsed segments (text is parsed here).
    fn resolve(&self) -> RecursiveDescentSegments {
        match self {
            PathInput::Text(text) => parse_path(&RegexPathTokenizer, text),
            PathInput::Segment(segment) => RecursiveDescentSegments(vec![(*segment).clone()]),
            PathInput::Segments(segments) => (*segments).clone(),
        }
    }
}

/// The manipulation entrypoint: a source value (borrowed or owned) plus an
/// optional schema, with `get` / `set` / `delete` / `for_each` methods taking
/// a JSONPath in any [`PathInput`] representation. Thin wrapper over the pure
/// application use cases.
pub struct DynObject<'a> {
    source: Source<'a>,
    schema: Option<Schema>,
}

impl<'a> DynObject<'a> {
    /// Wrap a borrowed mutable source.
    pub fn new(source: &'a mut dyn Reflect) -> Self {
        DynObject {
            source: Source::Borrowed(source),
            schema: None,
        }
    }

    /// Wrap a shared (read-only) source; `set`/`delete` on it fail.
    pub fn read_only(source: &'a dyn Reflect) -> Self {
        DynObject {
            source: Source::Shared(source),
            schema: None,
        }
    }

    /// Wrap an owned source (no external owner needed).
    pub fn owned(source: Box<dyn Reflect>) -> Self {
        DynObject {
            source: Source::Owned(source),
            schema: None,
        }
    }

    /// Attach a schema (guides `set` coercion/creation).
    pub fn with_schema(mut self, schema: &'a Schema) -> Self {
        self.schema = Some(schema.clone());
        self
    }

    /// Attach an optional schema.
    pub fn with_schema_opt(mut self, schema: Option<&'a Schema>) -> Self {
        self.schema = schema.cloned();
        self
    }

    /// Attach a schema node (wrapped in [`Schema::Node`]).
    pub fn with_schema_node(mut self, node: &'a SchemaNode) -> Self {
        self.schema = Some(Schema::Node(node.clone()));
        self
    }

    /// The wrapped source as a shared reference.
    pub fn source(&self) -> &dyn Reflect {
        self.source.as_reflect()
    }

    /// The wrapped source as a mutable reference, or `None` when the source
    /// is shared (read-only).
    pub fn source_mut(&mut self) -> Option<&mut dyn Reflect> {
        self.source.as_reflect_mut()
    }

    /// Consume the wrapper, returning the source as an owned box.
    pub fn into_source(self) -> Box<dyn Reflect> {
        self.source.into_owned()
    }

    /// Get the value(s) at `path` (see [`get`]).
    pub fn get<'s, 'p>(
        &'s self,
        path: impl Into<PathInput<'p>>,
    ) -> Result<Vec<&'s dyn Reflect>, ManipulationError> {
        let segments = path.into().resolve();
        get(self.source.as_reflect(), &segments)
    }

    /// Set `value` at `path` (see [`set`]) using the attached schema; returns
    /// how many values were set.
    pub fn set<'p>(
        &mut self,
        path: impl Into<PathInput<'p>>,
        value: &dyn Reflect,
    ) -> Result<usize, ManipulationError> {
        self.set_with_schema(path, value, self.schema.clone())
    }

    /// Set `value` at `path` with a per-call schema (overrides the attached
    /// one for this call; see [`set`]); returns how many values were set.
    pub fn set_with_schema<'p>(
        &mut self,
        path: impl Into<PathInput<'p>>,
        value: &dyn Reflect,
        schema: Option<Schema>,
    ) -> Result<usize, ManipulationError> {
        let segments = path.into().resolve();
        let Some(source) = self.source.as_reflect_mut() else {
            return Err(ManipulationError::ValueAtPathSegmentInvalid {
                message: "cannot set on a read-only source (DynObject::read_only)".to_string(),
            });
        };
        set(source, &segments, value, schema.as_ref())
    }

    /// Delete the value(s) at `path` (see [`delete`]); returns how many were deleted.
    pub fn delete<'p>(
        &mut self,
        path: impl Into<PathInput<'p>>,
    ) -> Result<usize, ManipulationError> {
        let segments = path.into().resolve();
        let Some(source) = self.source.as_reflect_mut() else {
            return Err(ManipulationError::ValueAtPathSegmentInvalid {
                message: "cannot delete on a read-only source (DynObject::read_only)".to_string(),
            });
        };
        delete(source, &segments)
    }

    /// Call `callback` for each value found at `path` (see [`for_each`]); a
    /// `false` return terminates the walk.
    pub fn for_each<'p>(
        &self,
        path: impl Into<PathInput<'p>>,
        callback: &mut dyn FnMut(&dyn Reflect) -> bool,
    ) -> Result<(), ManipulationError> {
        let segments = path.into().resolve();
        for_each(self.source.as_reflect(), &segments, callback)
    }
}
