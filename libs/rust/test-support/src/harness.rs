//! The data-driven test harness (ADR 0004), ported from dyn-rt-rs-old-3's
//! `shared` crate: case tables are plain Rust structs built with constructor
//! macros, each suite is one `#[test]` looping over a `yield_test_data()`
//! table, outcomes are collected in a [`TestReport`] and reported once with
//! the case title and expected-vs-got — so the tables double as usage
//! examples and every run shows coverage (which cases ran and passed), not
//! just failures.

use std::fmt;

/// A basic test title from a case's `test_title` field, based on whether it
/// was set. Expects `test_title` to be an `Option<String>`.
#[macro_export]
macro_rules! new_test_title {
    ($test_number:expr, $test_title:expr) => {
        match &$test_title {
            Some(title) => format!("Test case -> {} : [[[ {} ]]]", $test_number, title),
            None => format!("Test case -> {}", $test_number),
        }
    };
}

/// One recorded case: its title plus the failure detail, if it failed.
struct CaseOutcome {
    title: String,
    failure: Option<String>,
}

/// General-purpose suite report (ADR 0004): records every case as passed or
/// failed so a run can log both — failures panic once with a summary, passes
/// are listed as coverage.
///
/// Loop idiom inside a suite:
///
/// ```ignore
/// let mut report = TestReport::new();
/// for (test_index, case) in test_data.iter().enumerate() {
///     let title = new_test_title!(test_index + 1, case.meta.test_title);
///     report.begin_case(title.clone());
///     // ... on mismatch: report.fail_case(format!("expected {:?}, got {:?}", ...));
/// }
/// report.finish();
/// ```
pub struct TestReport {
    cases: Vec<CaseOutcome>,
}

impl TestReport {
    pub fn new() -> Self {
        Self { cases: Vec::new() }
    }

    /// Register the case currently being checked. Call once per loop
    /// iteration, right after computing the case title; the case is a pass
    /// unless [`TestReport::fail_case`] marks it failed.
    pub fn begin_case(&mut self, title: impl Into<String>) {
        self.cases.push(CaseOutcome {
            title: title.into(),
            failure: None,
        });
    }

    /// Mark the most recently begun case as failed with `detail`.
    pub fn fail_case(&mut self, detail: impl fmt::Display) {
        match self.cases.last_mut() {
            Some(case) => case.failure = Some(detail.to_string()),
            None => panic!("fail_case called before begin_case"),
        }
    }

    /// Record an already-resolved passing case (for ad-hoc checks outside the
    /// main loop).
    pub fn pass(&mut self, title: impl Into<String>) {
        self.begin_case(title);
    }

    /// Record an already-resolved failing case (for ad-hoc checks outside the
    /// main loop).
    pub fn fail(&mut self, title: impl Into<String>, detail: impl fmt::Display) {
        self.begin_case(title);
        self.fail_case(detail);
    }

    /// Finish the suite: panic once with the failure summary (plus the passed
    /// coverage) when any case failed, otherwise log the full coverage.
    pub fn finish(self) {
        let total = self.cases.len();
        self.finish_with_total(total);
    }

    /// Same as [`TestReport::finish`], with an explicit case total for callers
    /// that track cases outside the report.
    pub fn finish_with_total(self, total: usize) {
        let failed_lines: Vec<String> = self
            .cases
            .iter()
            .filter_map(|case| {
                case.failure
                    .as_ref()
                    .map(|detail| format!("{} || {}", case.title, detail))
            })
            .collect();
        let passed_titles: Vec<&str> = self
            .cases
            .iter()
            .filter(|case| case.failure.is_none())
            .map(|case| case.title.as_str())
            .collect();

        if !failed_lines.is_empty() {
            panic!(
                "\n--- {} / {} tests failed ---\n{}\n--- {} passed ---\n{}\n",
                failed_lines.len(),
                total,
                failed_lines.join("\n"),
                passed_titles.len(),
                passed_titles.join("\n")
            );
        }

        if passed_titles.is_empty() {
            println!("\n--- {} / {} tests passed ---\n", total, total);
        } else {
            println!(
                "\n--- {} / {} tests passed ---\n{}\n",
                passed_titles.len(),
                total,
                passed_titles.join("\n")
            );
        }
    }
}

impl Default for TestReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Old-3 style suite exit: panic once with a summary when any case failed.
///
/// Kept as a thin delegate over [`TestReport`] for suites that collect
/// preformatted `"title || detail"` failure lines themselves.
pub fn report_failures(failures: Vec<String>, total: usize) {
    let mut report = TestReport::new();
    for line in failures {
        match line.split_once(" || ") {
            Some((title, detail)) => report.fail(title.to_string(), detail.to_string()),
            None => report.fail(line, ""),
        }
    }
    report.finish_with_total(total);
}
