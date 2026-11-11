//! dyn-rt json application: the parse use cases.
//!
//! Orchestration only — all matching mechanics come from the
//! [`PathTokenizer`] port, so this crate stays free of regex/serde and the
//! steps stay explicit and in order (ADR 0005):
//!
//! 1. split the path by the recursive descent operator `..`
//! 2. split each part by dot notation `.`
//! 3. split each part into bracket contents and plain text
//! 4. extract one [`PathSegment`] per token

use dyn_rt_json_domain::path::{
    AccessStrategy, PathSegment, PathSelector, RecursiveDescentSegment, RecursiveDescentSegments,
};
use dyn_rt_json_domain::ports::PathTokenizer;

/// Parse a JSONPath string into its deconstructed AST.
///
/// Example: `$.store.book[0].title` parses into two-part descent groups as
/// held by [`RecursiveDescentSegments`].
pub fn parse_path(tokenizer: &dyn PathTokenizer, path: &str) -> RecursiveDescentSegments {
    let mut segments = RecursiveDescentSegments(Vec::new());

    // Step 1: split by recursive descent `..`
    for descent_part in tokenizer.split_recursive_descent(path) {
        let mut group = RecursiveDescentSegment(Vec::new());

        // Step 2: split by dot notation `.`
        for dot_part in tokenizer.split_dot_notation(descent_part) {
            // Step 3: split into bracket contents and plain text
            for token in tokenizer.split_brackets(dot_part) {
                // Step 4: extract the segment for this token
                group.0.push(extract_segment(tokenizer, token));
            }
        }

        segments.0.push(group);
    }

    segments
}

/// Extract the [`PathSegment`] for a single token (one item out of step 3).
///
/// The checks run in a fixed, explicit order: root, wildcard, union, quoted
/// key, positional index, slice, plain key, unrecognized.
pub fn extract_segment(tokenizer: &dyn PathTokenizer, token: &str) -> PathSegment {
    // JSONPath root
    if token == "$" {
        return PathSegment {
            access_strategy: AccessStrategy::Mixed,
            selector: PathSelector::Root,
        };
    }

    // JSONPath wildcard
    if token == "*" {
        return PathSegment {
            access_strategy: AccessStrategy::Mixed,
            selector: PathSelector::Wildcard,
        };
    }

    // Union selector (more than one comma-separated member)
    let union_parts = tokenizer.split_union(token);
    if union_parts.len() > 1 {
        return extract_union_segment(tokenizer, &union_parts);
    }

    // Quoted text key
    if let Some(key) = tokenizer.extract_quoted(token) {
        return PathSegment {
            access_strategy: AccessStrategy::Named,
            selector: PathSelector::Key(key),
        };
    }

    // Positional index
    if let Some(index) = tokenizer.parse_index(token) {
        return PathSegment {
            access_strategy: AccessStrategy::Positional,
            selector: PathSelector::Index(index),
        };
    }

    // Slice selector
    if let Some(slice) = tokenizer.parse_slice(token) {
        return PathSegment {
            access_strategy: AccessStrategy::Positional,
            selector: PathSelector::Slice(slice),
        };
    }

    // Plain text key
    let trimmed = token.trim();
    if !trimmed.is_empty() {
        return PathSegment {
            access_strategy: AccessStrategy::Named,
            selector: PathSelector::Key(trimmed.to_string()),
        };
    }

    // Nothing matched
    PathSegment::default()
}

/// Extract a union segment from its comma-separated member tokens.
///
/// The union's access strategy is Mixed when any member is Mixed or when
/// members mix Positional and Named; otherwise it inherits the single
/// member kind present.
fn extract_union_segment(tokenizer: &dyn PathTokenizer, union_parts: &[&str]) -> PathSegment {
    let mut members: Vec<PathSegment> = Vec::with_capacity(union_parts.len());
    let (mut has_mixed, mut has_positional, mut has_named) = (false, false, false);

    for part in union_parts {
        let member = extract_segment(tokenizer, part);
        match member.access_strategy {
            AccessStrategy::Mixed => has_mixed = true,
            AccessStrategy::Positional => has_positional = true,
            AccessStrategy::Named => has_named = true,
        }
        members.push(member);
    }

    let access_strategy = if has_mixed || (has_positional && has_named) {
        AccessStrategy::Mixed
    } else if has_positional {
        AccessStrategy::Positional
    } else if has_named {
        AccessStrategy::Named
    } else {
        AccessStrategy::default()
    };

    PathSegment {
        access_strategy,
        selector: PathSelector::Union(members),
    }
}
