//! The data-driven test harness (ADR 0004), ported from dyn-rt-rs-old-3's
//! `shared` crate: case tables are plain Rust structs built with constructor
//! macros, each suite is one `#[test]` looping over a `yield_test_data()`
//! table, failures are collected and reported once with the case title and
//! expected-vs-got — so the tables double as usage examples.

/// Per-case metadata. `test_title` shows up in failure output.
#[derive(Debug, Default)]
pub struct TestMetadata {
    pub test_title: Option<String>,
    pub log_errors_if_expected_not_ok: bool,
}

/// Helper for creating new [`TestMetadata`].
#[macro_export]
macro_rules! new_test_metadata {
    ($($field_name:ident = $field_value:expr),* $(,)?) => {
        $crate::TestMetadata {
            $(
                $field_name: $field_value,
            )*
            ..Default::default()
        }
    };
}

/// A basic test title based on whether the `test_title` in the [`TestMetadata`] was set.
#[macro_export]
macro_rules! new_test_title {
    // Title set from literal.
    ($test_number:expr, $metadata_test_title:literal) => {
        format!(
            "Test case -> {} : [[[ {} ]]]",
            $test_number, $metadata_test_title
        )
    };

    // Title set from `test_title` in [`TestMetadata`].
    //
    // Expects metadata_test_title to be an `Option<String>`.
    ($test_number:expr, $metadata_test_title:expr) => {
        match &$metadata_test_title {
            Some(title) => format!("Test case -> {} : [[[ {} ]]]", $test_number, title),
            None => format!("Test case -> {}", $test_number),
        }
    };
}

/// Old-3 style suite exit: panic once with a summary when any case failed.
pub fn report_failures(failures: Vec<String>, total: usize) {
    if !failures.is_empty() {
        panic!(
            "\n--- {} / {} tests failed ---\n{}\n",
            failures.len(),
            total,
            failures.join("\n")
        );
    }
}
