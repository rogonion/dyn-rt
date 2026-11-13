//! Data-driven suite for the json-manipulation DOMAIN layer (ADR 0004/0009):
//! the path cursor (src/domain.rs) and the error vocabulary (src/error.rs).

use dyn_rt_json_manipulation::domain::PathCursor;
use dyn_rt_json_manipulation::error::ManipulationError;
use dyn_rt_json_path::{
    RecursiveDescentSegment, RecursiveDescentSegments, json_path_key_selector_segment,
    json_path_root_segment,
};
use dyn_rt_test_support::{TestReport, new_test_title};

/// Builds a synthetic parsed path with `groups` groups, the first group
/// holding `first_group_len` segments (root + keys).
fn synthetic_path(groups: usize, first_group_len: usize) -> RecursiveDescentSegments {
    let mut all = Vec::new();
    for g in 0..groups {
        let len = if g == 0 { first_group_len } else { 1 };
        let mut segments = Vec::new();
        if g == 0 && len > 0 {
            segments.push(json_path_root_segment!());
        }
        for _ in segments.len()..len {
            segments.push(json_path_key_selector_segment!("k"));
        }
        all.push(RecursiveDescentSegment(segments));
    }
    RecursiveDescentSegments(all)
}

struct CursorCase {
    test_title: Option<String>,
    path: RecursiveDescentSegments,
    /// advance the cursor this many times within the group before checking
    advance: usize,
    expect_terminal: bool,
    expect_group_end: bool,
}

macro_rules! cursor_case {
    ($path:expr, $advance:expr, $terminal:expr, $group_end:expr $(, test_title = $title:expr)?) => {
        CursorCase {
            test_title: None $(.or(Some($title.to_string())))?,
            path: $path,
            advance: $advance,
            expect_terminal: $terminal,
            expect_group_end: $group_end,
        }
    };
}

#[test]
fn path_cursor_cases() {
    let test_data: Vec<CursorCase> = vec![
        cursor_case!(
            synthetic_path(1, 1),
            0,
            true,
            true,
            test_title = "lone root segment is terminal and group end"
        ),
        cursor_case!(
            synthetic_path(1, 3),
            0,
            false,
            false,
            test_title = "first of three segments in a single group"
        ),
        cursor_case!(
            synthetic_path(1, 3),
            2,
            true,
            true,
            test_title = "last segment of a single group is terminal and group end"
        ),
        cursor_case!(
            synthetic_path(2, 2),
            1,
            false,
            true,
            test_title = "last segment of the first group is a group end (.. boundary follows) but not terminal"
        ),
    ];

    let mut report = TestReport::new(
        "path_cursor_cases",
        "PathCursor terminal/group-end transitions (src/domain.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let mut cursor = PathCursor::start(case.path.0.len());
        for _ in 0..case.advance {
            cursor = cursor.next_in_group();
        }
        let terminal = cursor.is_terminal(&case.path);
        let group_end = cursor.is_group_end(&case.path);
        if terminal != case.expect_terminal || group_end != case.expect_group_end {
            report.fail_case(format!(
                "expected terminal={} group_end={}, got terminal={} group_end={}",
                case.expect_terminal, case.expect_group_end, terminal, group_end
            ));
        }
    }
    report.finish();
}

struct ErrorCase {
    test_title: Option<String>,
    error: ManipulationError,
    expected_display: &'static str,
}

macro_rules! error_case {
    ($error:expr, $expected:expr $(, test_title = $title:expr)?) => {
        ErrorCase {
            test_title: None $(.or(Some($title.to_string())))?,
            error: $error,
            expected_display: $expected,
        }
    };
}

#[test]
fn error_display_cases() {
    let test_data: Vec<ErrorCase> = vec![
        error_case!(
            ManipulationError::PathSegmentInvalid {
                message: "boom".to_string()
            },
            "path segment invalid: boom",
            test_title = "path segment invalid renders with prefix"
        ),
        error_case!(
            ManipulationError::ValueAtPathSegmentInvalid {
                message: "gone".to_string()
            },
            "value at path segment invalid: gone",
            test_title = "value invalid renders with prefix"
        ),
        error_case!(
            ManipulationError::ConversionFailed {
                message: "nope".to_string()
            },
            "conversion failed: nope",
            test_title = "conversion failure renders with prefix"
        ),
    ];

    let mut report = TestReport::new(
        "error_display_cases",
        "ManipulationError Display (src/error.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        if case.error.to_string() != case.expected_display {
            report.fail_case(format!(
                "expected {:?}, got {:?}",
                case.expected_display,
                case.error.to_string()
            ));
        }
    }
    report.finish();
}
