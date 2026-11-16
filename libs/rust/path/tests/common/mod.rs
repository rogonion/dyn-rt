//! Shared case-table types for the path suites — old-3 style: each case
//! is a plain struct built by a constructor macro with an optional
//! `test_title`, and each suite is one `#[test]` looping a `yield_*_data()`
//! table (ADR 0004/0009). Included per test crate via `mod common;` (or
//! `#[path]` from the adapters crate root).
#![allow(dead_code, unused_macros, unused_imports)]

use dyn_rt_path::domain::{PathSegment, RecursiveDescentSegments};

/// Case for the three split steps: source path -> expected parts.
pub struct SplitCase {
    pub test_title: Option<String>,
    pub source: &'static str,
    pub expected: Vec<&'static str>,
}

macro_rules! split_case {
    ($source:literal, $($expected:literal),+ $(, test_title = $title:expr)?) => {
        SplitCase {
test_title: None $(.or(Some($title.to_string())))?,
            source: $source,
            expected: vec![
                $(
                    $expected,
                )+
            ],
        }
    };
}
pub(crate) use split_case;

/// Case for extract_segment: source token -> expected PathSegment.
pub struct ExtractCase {
    pub test_title: Option<String>,
    pub source: &'static str,
    pub expected: PathSegment,
}

macro_rules! extract_case {
    ($source:literal, $segment:expr $(, test_title = $title:expr)?) => {
        ExtractCase {
test_title: None $(.or(Some($title.to_string())))?,
            source: $source,
            expected: $segment,
        }
    };
}
pub(crate) use extract_case;

/// Case for parse_path: source path -> expected AST.
pub struct ParseCase {
    pub test_title: Option<String>,
    pub source: &'static str,
    pub expected: RecursiveDescentSegments,
}

macro_rules! parse_case {
    ($source:literal, $segments:expr $(, test_title = $title:expr)?) => {
        ParseCase {
test_title: None $(.or(Some($title.to_string())))?,
            source: $source,
            expected: $segments,
        }
    };
}
pub(crate) use parse_case;

/// Case for Display rendering: AST -> expected path string (round-trip).
pub struct RenderCase {
    pub test_title: Option<String>,
    pub segments: RecursiveDescentSegments,
    pub roundtrip: &'static str,
}

macro_rules! render_case {
    ($segments:expr, roundtrip = $roundtrip:literal $(, test_title = $title:expr)?) => {
        RenderCase {
test_title: None $(.or(Some($title.to_string())))?,
            segments: $segments,
            roundtrip: $roundtrip,
        }
    };
}
pub(crate) use render_case;
