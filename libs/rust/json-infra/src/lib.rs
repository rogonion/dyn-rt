//! dyn-rt json infra: adapters for the json core.
//!
//! This is the ONLY json crate (besides apps) allowed to use `regex` (ADR
//! 0002). Each regex is a named, single-purpose static applied in one
//! explicit step of the parse pipeline (ADR 0005) — ported from
//! dyn-rt-rs-old-3, which is the logic reference for the parser.

use std::sync::LazyLock;

use regex::Regex;

use dyn_rt_json_domain::path::SliceSelector;
use dyn_rt_json_domain::ports::PathTokenizer;

/// Matches bracketed selector expressions `[...]` (ignoring internal dots)
/// OR the standalone recursive descent operator `..`.
///
/// Used in step 1 (`split_recursive_descent`).
pub static RECURSIVE_DESCENT_SPLIT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        \[(?: '[^'\\]*(?:\\.[^'\\]*)*' | "[^"\\]*(?:\\.[^"\\]*)*" | [^]] )+ \]
        | (?P<descent> \.{2} )
        "#,
    )
    .unwrap()
});

pub const RECURSIVE_DESCENT_SPLIT_REGEX_DESCENT_CAPTURE_NAME: &str = "descent";

/// Matches bracketed selector expressions `[...]` (ignoring internal dots)
/// OR the standalone dot notation operator `.`.
///
/// Used in step 2 (`split_dot_notation`).
pub static MEMBER_DOT_NOTATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        \[ (?: '[^'\\]*(?:\\.[^'\\]*)*' | "[^"\\]*(?:\\.[^"\\]*)*" | [^]] )+ \]
        | (?P<dot> \. )
        "#,
    )
    .unwrap()
});

pub const MEMBER_DOT_NOTATION_REGEX_CAPTURE_NAME: &str = "dot";

/// Splits a part into bracket contents (`ibracket`) and non-bracket text
/// (`obracket`).
///
/// Used in step 3 (`split_brackets`).
pub static BRACKET_SPLIT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        \[ (?P<ibracket> (?: '[^'\\]*(?:\\.[^'\\]*)*' | "[^"\\]*(?:\\.[^"\\]*)*" | [^]] )+ ) \]
        | (?P<obracket> [^\[]+ )
        "#,
    )
    .unwrap()
});

pub const BRACKET_SPLIT_REGEX_OUT_BRACKET_CAPTURE_NAME: &str = "obracket";
pub const BRACKET_SPLIT_REGEX_IN_BRACKET_CAPTURE_NAME: &str = "ibracket";

/// Matches a positional index: an optionally negative integer, anchored.
pub static POSITIONAL_INDEX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?P<position> ^ -?\d+ $ )
        "#,
    )
    .unwrap()
});

pub const POSITIONAL_INDEX_REGEX_CAPTURE_NAME: &str = "position";

/// Matches a linear collection slice `start:end:step` (all parts optional,
/// negatives allowed), e.g. `:`, `1:5`, `::2`, `-1:-5:-1`.
pub static SLICE_SELECTOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?P<start>-?\d*):(?P<end>-?\d*)(?:\:(?P<step>-?\d*))?
        "#,
    )
    .unwrap()
});

pub const SLICE_SELECTOR_REGEX_START_CAPTURE_NAME: &str = "start";
pub const SLICE_SELECTOR_REGEX_END_CAPTURE_NAME: &str = "end";
pub const SLICE_SELECTOR_REGEX_STEP_CAPTURE_NAME: &str = "step";

/// Matches the union splitter `,` while ignoring commas inside quotes.
///
/// Used in step 4a (`split_union`).
pub static UNION_SPLIT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        ' [^'\\]* (?: \\. [^'\\]* )* '
        | " [^"\\]* (?: \\. [^"\\]* )* "
        | ` [^`\\]* (?: \\. [^`\\]* )* `
        | (?P<comma> , )
        "#,
    )
    .unwrap()
});

pub const UNION_SPLIT_REGEX_CAPTURE_NAME: &str = "comma";

/// Matches a fully quoted string and captures the unquoted content per quote
/// style: single, double, backtick.
pub static QUOTE_EXTRACTOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        ^ (?:
            ' (?P<single>   [^'\\]* (?: \\. [^'\\]* )* ) '
          | " (?P<double>   [^"\\]* (?: \\. [^"\\]* )* ) "
          | ` (?P<backtick> [^`\\]* (?: \\. [^`\\]* )* ) `
        ) $
        "#,
    )
    .unwrap()
});

pub const QUOTE_EXTRACTOR_REGEX_SINGLE_CAPTURE_NAME: &str = "single";
pub const QUOTE_EXTRACTOR_REGEX_DOUBLE_CAPTURE_NAME: &str = "double";
pub const QUOTE_EXTRACTOR_REGEX_BACKTICK_CAPTURE_NAME: &str = "backtick";

/// The default [`PathTokenizer`]: the named regexes above applied in the
/// explicit step order of the parse pipeline.
pub struct RegexPathTokenizer;

impl PathTokenizer for RegexPathTokenizer {
    fn split_recursive_descent<'a>(&self, path: &'a str) -> Vec<&'a str> {
        let mut segments: Vec<&str> = Vec::new();
        let mut last_descent_end_index: usize = 0;

        for capture in RECURSIVE_DESCENT_SPLIT_REGEX.captures_iter(path) {
            if let Some(descent_capture) =
                capture.name(RECURSIVE_DESCENT_SPLIT_REGEX_DESCENT_CAPTURE_NAME)
            {
                segments.push(&path[last_descent_end_index..descent_capture.start()]);
                last_descent_end_index = descent_capture.end();
            }
        }

        // append the final segment
        segments.push(&path[last_descent_end_index..]);

        segments
    }

    fn split_dot_notation<'a>(&self, part: &'a str) -> Vec<&'a str> {
        let mut segments: Vec<&str> = Vec::new();
        let mut last_dot_end_index: usize = 0;

        for capture in MEMBER_DOT_NOTATION_REGEX.captures_iter(part) {
            if let Some(dot_capture) = capture.name(MEMBER_DOT_NOTATION_REGEX_CAPTURE_NAME) {
                segments.push(&part[last_dot_end_index..dot_capture.start()]);
                last_dot_end_index = dot_capture.end();
            }
        }

        // append the final segment
        segments.push(&part[last_dot_end_index..]);

        segments
    }

    fn split_brackets<'a>(&self, part: &'a str) -> Vec<&'a str> {
        let mut segments: Vec<&str> = Vec::new();

        for capture in BRACKET_SPLIT_REGEX.captures_iter(part) {
            if let Some(in_bracket) = capture.name(BRACKET_SPLIT_REGEX_IN_BRACKET_CAPTURE_NAME) {
                segments.push(in_bracket.as_str());
                continue;
            }

            if let Some(out_bracket) = capture.name(BRACKET_SPLIT_REGEX_OUT_BRACKET_CAPTURE_NAME) {
                let trimmed = out_bracket.as_str().trim();
                if !trimmed.is_empty() {
                    segments.push(trimmed);
                }
            }
        }

        segments
    }

    fn split_union<'a>(&self, content: &'a str) -> Vec<&'a str> {
        let mut parts: Vec<&str> = Vec::new();
        let mut last_union_end_index: usize = 0;

        for capture in UNION_SPLIT_REGEX.captures_iter(content) {
            if let Some(comma_capture) = capture.name(UNION_SPLIT_REGEX_CAPTURE_NAME) {
                parts.push(&content[last_union_end_index..comma_capture.start()]);
                last_union_end_index = comma_capture.end();
            }
        }

        // append the final part
        parts.push(&content[last_union_end_index..]);

        parts
    }

    fn extract_quoted(&self, token: &str) -> Option<String> {
        let capture = QUOTE_EXTRACTOR_REGEX.captures(token)?;

        if let Some(quoted) = capture.name(QUOTE_EXTRACTOR_REGEX_SINGLE_CAPTURE_NAME) {
            return Some(quoted.as_str().to_string());
        }
        if let Some(quoted) = capture.name(QUOTE_EXTRACTOR_REGEX_DOUBLE_CAPTURE_NAME) {
            return Some(quoted.as_str().to_string());
        }
        if let Some(quoted) = capture.name(QUOTE_EXTRACTOR_REGEX_BACKTICK_CAPTURE_NAME) {
            return Some(quoted.as_str().to_string());
        }

        None
    }

    fn parse_index(&self, token: &str) -> Option<isize> {
        let capture = POSITIONAL_INDEX_REGEX.captures(token)?;
        capture
            .name(POSITIONAL_INDEX_REGEX_CAPTURE_NAME)?
            .as_str()
            .parse::<isize>()
            .ok()
    }

    fn parse_slice(&self, token: &str) -> Option<SliceSelector> {
        let capture = SLICE_SELECTOR_REGEX.captures(token)?;
        let mut slice = SliceSelector::default();

        if let Some(start_match) = capture.name(SLICE_SELECTOR_REGEX_START_CAPTURE_NAME)
            && let Ok(num) = start_match.as_str().parse::<isize>()
        {
            slice.start = Some(num);
        }

        if let Some(end_match) = capture.name(SLICE_SELECTOR_REGEX_END_CAPTURE_NAME)
            && let Ok(num) = end_match.as_str().parse::<isize>()
        {
            slice.end = Some(num);
        }

        if let Some(step_match) = capture.name(SLICE_SELECTOR_REGEX_STEP_CAPTURE_NAME)
            && let Ok(num) = step_match.as_str().parse::<isize>()
        {
            slice.step = Some(num);
        }

        if slice.start.is_some() || slice.end.is_some() || slice.step.is_some() {
            return Some(slice);
        }

        None
    }
}
