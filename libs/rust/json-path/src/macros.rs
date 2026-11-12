//! Constructor macros for writing path ASTs (and their test tables) in the
//! shape of the path itself — ported from dyn-rt-rs-old-3's
//! `json_path_*_segment!` / `new_recursive_descent_segment(s)!` macros
//! (ADR 0004/0005: declarative macros are the sanctioned way to keep
//! data-driven tables readable).

#[macro_export]
macro_rules! json_path_root_segment {
    () => {
        $crate::domain::PathSegment {
            access_strategy: $crate::domain::AccessStrategy::Mixed,
            selector: $crate::domain::PathSelector::Root,
        }
    };
}

#[macro_export]
macro_rules! json_path_wildcard_segment {
    () => {
        $crate::domain::PathSegment {
            access_strategy: $crate::domain::AccessStrategy::Mixed,
            selector: $crate::domain::PathSelector::Wildcard,
        }
    };
}

#[macro_export]
macro_rules! json_path_key_selector_segment {
    ($key:literal) => {
        $crate::domain::PathSegment {
            access_strategy: $crate::domain::AccessStrategy::Named,
            selector: $crate::domain::PathSelector::Key($key.to_string()),
        }
    };
}

#[macro_export]
macro_rules! json_path_index_selector_segment {
    ($index:expr) => {
        $crate::domain::PathSegment {
            access_strategy: $crate::domain::AccessStrategy::Positional,
            selector: $crate::domain::PathSelector::Index($index),
        }
    };
}

/// One explicit arm per slice shape (ADR 0005). Note `::` is a single token,
/// so `start::step` needs its own arm.
#[macro_export]
macro_rules! json_path_slice_selector_segment {
    // start:end:step
    ($start:literal : $end:literal : $step:literal) => {
        $crate::json_path_slice_selector_segment!(@segment Some($start), Some($end), Some($step))
    };
    // start:end
    ($start:literal : $end:literal) => {
        $crate::json_path_slice_selector_segment!(@segment Some($start), Some($end), None)
    };
    // start::step
    ($start:literal :: $step:literal) => {
        $crate::json_path_slice_selector_segment!(@segment Some($start), None, Some($step))
    };
    // start:
    ($start:literal :) => {
        $crate::json_path_slice_selector_segment!(@segment Some($start), None, None)
    };
    // :end:step
    (: $end:literal : $step:literal) => {
        $crate::json_path_slice_selector_segment!(@segment None, Some($end), Some($step))
    };
    // ::step
    (:: $step:literal) => {
        $crate::json_path_slice_selector_segment!(@segment None, None, Some($step))
    };
    // :end
    (: $end:literal) => {
        $crate::json_path_slice_selector_segment!(@segment None, Some($end), None)
    };
    // :
    (:) => {
        $crate::json_path_slice_selector_segment!(@segment None, None, None)
    };
    (@segment $start:expr, $end:expr, $step:expr) => {
        $crate::domain::PathSegment {
            access_strategy: $crate::domain::AccessStrategy::Positional,
            selector: $crate::domain::PathSelector::Slice(
                $crate::domain::SliceSelector {
                    start: $start,
                    end: $end,
                    step: $step,
                }
            ),
        }
    };
}

#[macro_export]
macro_rules! json_path_union_selector_segment {
    ($access_strategy:expr, $($union_option:expr),* $(,)?) => {
        $crate::domain::PathSegment {
            access_strategy: $access_strategy,
            selector: $crate::domain::PathSelector::Union(vec![
                $(
                    $union_option,
                )*
            ]),
        }
    };
}

#[macro_export]
macro_rules! new_recursive_descent_segment {
    ($($segment:expr),* $(,)?) => {
        $crate::domain::RecursiveDescentSegment(vec![
            $(
                $segment,
            )*
        ])
    };
}

#[macro_export]
macro_rules! new_recursive_descent_segments {
    ($($segment:expr),* $(,)?) => {
        $crate::domain::RecursiveDescentSegments(vec![
            $(
                $segment,
            )*
        ])
    };
}
