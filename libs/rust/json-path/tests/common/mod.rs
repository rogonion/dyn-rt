#![allow(dead_code, unused_macros, unused_imports)]

//! Shared case-table types for the json-path suites — old-3 style: each case
//! is a plain struct built by a constructor macro with an optional
//! `test_title`, and each suite is one `#[test]` looping a `yield_test_data()`
//! table (ADR 0004).

use dyn_rt_json_path::domain::{PathSegment, RecursiveDescentSegments};
use dyn_rt_test_support::TestMetadata;

/// Case for the three split steps: source path -> expected parts.
pub struct SplitCase {
    pub meta: TestMetadata,
    pub source: &'static str,
    pub expected: Vec<&'static str>,
}

macro_rules! split_case {
    ($source:literal, $($expected:literal),+ $(, test_title = $title:expr)?) => {
        SplitCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
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
    pub meta: TestMetadata,
    pub source: &'static str,
    pub expected: PathSegment,
}

macro_rules! extract_case {
    ($source:literal, $segment:expr $(, test_title = $title:expr)?) => {
        ExtractCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            source: $source,
            expected: $segment,
        }
    };
}
pub(crate) use extract_case;

/// Case for full parse: source path -> expected AST + expected round-trip.
pub struct ParseCase {
    pub meta: TestMetadata,
    pub source: &'static str,
    pub expected: RecursiveDescentSegments,
    pub roundtrip: &'static str,
}

macro_rules! parse_case {
    ($source:literal, $segments:expr, roundtrip = $roundtrip:literal $(, test_title = $title:expr)?) => {
        ParseCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
            source: $source,
            expected: $segments,
            roundtrip: $roundtrip,
        }
    };
}
pub(crate) use parse_case;
