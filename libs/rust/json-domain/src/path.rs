//! JSONPath AST: the deconstructed form of a JSONPath string.
//!
//! A path like `$.store.book[0].title` parses into [`RecursiveDescentSegments`]:
//! a list of [`RecursiveDescentSegment`]s separated by the recursive descent
//! operator `..`, each holding a linear sequence of [`PathSegment`]s.
//!
//! The AST shape and selector rendering follow the Rust reference port
//! (dyn-rt-rs-old-3). The descent-group joining rules in [`fmt::Display`]
//! follow go's `String()` (the one piece old-3 left unimplemented): keys that
//! are plain identifiers join with `.`, everything else (bracketed keys,
//! indices, slices, unions) attaches directly, and descent groups join
//! with `..`.

use std::fmt;

/// A JSONPath string, e.g. `$.store.book[0].title`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JsonPath(pub String);

impl JsonPath {
    pub fn new(path: impl Into<String>) -> Self {
        JsonPath(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The deconstructed JSONPath: segments split by the recursive descent
/// operator `..`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecursiveDescentSegments(pub Vec<RecursiveDescentSegment>);

/// A sequence of path segments directly connected (by `.` or brackets)
/// without any recursive descent operator.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecursiveDescentSegment(pub Vec<PathSegment>);

/// A single atomic segment of a JSONPath.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathSegment {
    pub access_strategy: AccessStrategy,
    pub selector: PathSelector,
}

/// Hint for how the segment's value is accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessStrategy {
    /// Can be named or positional (root, wildcard, mixed unions).
    #[default]
    Mixed,
    /// Accessed by key/name.
    Named,
    /// Accessed by sequence index.
    Positional,
}

/// The selector of a single path segment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PathSelector {
    /// A token that did not match any known selector form.
    #[default]
    Unrecognized,
    /// The root `$`.
    Root,
    /// The wildcard `*`.
    Wildcard,
    /// A named key.
    Key(String),
    /// A positional index, e.g. `[0]` or `[-1]`.
    Index(isize),
    /// A slice selector, e.g. `[1:5:2]`.
    Slice(SliceSelector),
    /// A union selector, e.g. `['a','b',0]`.
    Union(Vec<PathSegment>),
}

/// A linear-collection slice `[start:end:step]`; each part is optional.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SliceSelector {
    pub start: Option<isize>,
    pub end: Option<isize>,
    pub step: Option<isize>,
}

/// A key renders bare (dot notation) only if it starts with an ASCII letter
/// and contains only ASCII letters, digits, or underscores; otherwise it needs
/// the bracket-quote form `['key']`. Written as an explicit char check (no
/// regex) per ADR 0005.
fn key_needs_brackets(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return true,
    }
    chars.any(|c| !(c.is_ascii_alphanumeric() || c == '_'))
}

impl fmt::Display for SliceSelector {
    /// Renders `[start:end:step]`, keeping the colons that are needed to
    /// preserve positions, e.g. `[1:5:2]`, `[1:-5]`, `[:5]`, `[::2]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::from("[");
        if let Some(start) = self.start {
            s.push_str(&start.to_string());
            s.push(':');
        }
        if let Some(end) = self.end {
            if self.start.is_none() {
                s.push(':');
            }
            s.push_str(&end.to_string());
        }
        if let Some(step) = self.step {
            if self.start.is_none() && self.end.is_none() {
                s.push(':');
            }
            s.push(':');
            s.push_str(&step.to_string());
        }
        s.push(']');
        f.write_str(&s)
    }
}

impl fmt::Display for PathSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSelector::Unrecognized => Ok(()),
            PathSelector::Root => f.write_str("$"),
            PathSelector::Wildcard => f.write_str("*"),
            PathSelector::Key(key) => {
                if key_needs_brackets(key) {
                    write!(f, "['{}']", key)
                } else {
                    f.write_str(key)
                }
            }
            PathSelector::Index(index) => write!(f, "[{}]", index),
            PathSelector::Slice(slice) => write!(f, "{}", slice),
            PathSelector::Union(members) => {
                let parts: Vec<String> = members
                    .iter()
                    .map(|member| {
                        let rendered = member.selector.to_string();
                        // Strip one layer of brackets so members do not double-wrap.
                        if rendered.starts_with('[')
                            && rendered.ends_with(']')
                            && rendered.len() >= 2
                        {
                            rendered[1..rendered.len() - 1].to_string()
                        } else {
                            rendered
                        }
                    })
                    .collect();
                write!(f, "[{}]", parts.join(","))
            }
        }
    }
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.selector.fmt(f)
    }
}

impl fmt::Display for RecursiveDescentSegment {
    /// Joins the rendered segments: the first segment stands alone; a segment
    /// rendered in bracket form (`[...]`) attaches directly; anything else is
    /// joined with a `.` separator.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .0
            .iter()
            .map(|segment| segment.to_string())
            .filter(|rendered| !rendered.is_empty())
            .collect();
        if parts.is_empty() {
            return Ok(());
        }

        let mut result = parts[0].clone();
        let mut i = 1;
        while i < parts.len() {
            let part = &parts[i];
            let part_is_bracket = part.starts_with('[') && part.ends_with(']');
            if part_is_bracket {
                result.push_str(part);
            } else {
                if !result.ends_with('.') {
                    result.push('.');
                }
                result.push_str(part);
                if i != parts.len() - 1 {
                    let next = &parts[i + 1];
                    let next_is_bracket = next.starts_with('[') && next.ends_with(']');
                    if !next_is_bracket {
                        result.push('.');
                    }
                }
            }
            i += 1;
        }
        f.write_str(&result)
    }
}

impl fmt::Display for RecursiveDescentSegments {
    /// Joins the non-empty descent groups with `..`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let groups: Vec<String> = self
            .0
            .iter()
            .map(|segment| segment.to_string())
            .filter(|rendered| !rendered.is_empty())
            .collect();
        f.write_str(&groups.join(".."))
    }
}
