//! The data-driven test harness (ADR 0004), ported from dyn-rt-rs-old-3's
//! `shared` crate: case tables are plain Rust structs built with constructor
//! macros, each suite is one `#[test]` looping over a `yield_test_data()`
//! table, outcomes are collected in a [`TestReport`] and reported once with
//! the case title and expected-vs-got — so the tables double as usage
//! examples and every run shows coverage (which cases ran and passed), not
//! just failures.

use std::fmt;

/// A basic case title from a case's `test_title` field, based on whether it
/// was set. Expects `test_title` to be an `Option<String>`. The case number
/// is added by [`TestReport`] when it logs the case.
#[macro_export]
macro_rules! new_test_title {
    ($test_title:expr) => {
        match &$test_title {
            Some(title) => format!("[[[ {} ]]]", title),
            None => "(untitled case)".to_string(),
        }
    };
}

/// One recorded case: its title plus the failure detail, if it failed.
struct CaseOutcome {
    title: String,
    failure: Option<String>,
}

impl CaseOutcome {
    /// The per-case log line: `<number>/<total> | <title> | <outcome>`.
    /// Failures additionally carry their contextual detail.
    fn line(&self, number: usize, total: usize) -> String {
        match &self.failure {
            Some(detail) => format!(
                "Test case -> {}/{} : {} || FAIL || {}",
                number, total, self.title, detail
            ),
            None => format!("Test case -> {}/{} : {} || PASS", number, total, self.title),
        }
    }
}

/// General-purpose suite report (ADR 0004): a root suite title + description,
/// and one log line per case — pass or fail — so every run shows full
/// coverage. By default all cases are evaluated and a single panic at
/// `finish()` reports the failures; with [`TestReport::abort_on_failure`] the
/// first failing case panics immediately instead.
///
/// Loop idiom inside a suite:
///
/// ```ignore
/// let test_data = yield_test_data();
/// let mut report = TestReport::new("suite title", "what this suite covers");
/// for case in &test_data {
///     report.begin_case(new_test_title!(case.test_title));
///     // ... on mismatch: report.fail_case(format!("expected {:?}, got {:?}", ...));
/// }
/// report.finish();
/// ```
pub struct TestReport {
    suite_title: String,
    suite_description: String,
    cases: Vec<CaseOutcome>,
    abort_on_failure: bool,
}

impl TestReport {
    /// Start a suite report with a root `suite_title` and a
    /// `suite_description` of what the suite is about.
    pub fn new(suite_title: impl Into<String>, suite_description: impl Into<String>) -> Self {
        Self {
            suite_title: suite_title.into(),
            suite_description: suite_description.into(),
            cases: Vec::new(),
            abort_on_failure: false,
        }
    }

    /// Abort the suite on the first failing case (panic immediately from
    /// [`TestReport::fail_case`]) instead of collecting failures for the
    /// end-of-run panic.
    pub fn abort_on_failure(mut self) -> Self {
        self.abort_on_failure = true;
        self
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

    /// Mark the most recently begun case as failed with `detail`. With
    /// [`TestReport::abort_on_failure`] this panics immediately.
    pub fn fail_case(&mut self, detail: impl fmt::Display) {
        match self.cases.last_mut() {
            Some(case) => case.failure = Some(detail.to_string()),
            None => panic!("fail_case called before begin_case"),
        }
        if self.abort_on_failure {
            let total = self.cases.len();
            let line = self.cases.last().map(|case| case.line(total, total));
            panic!(
                "\n--- aborting suite on failed test case ---\n{}\n",
                line.unwrap_or_default()
            );
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

    /// Finish the suite: log the suite header and one line per case
    /// (`<number>/<total> | <title> | PASS|FAIL`), then panic once with the
    /// failure summary when any case failed.
    pub fn finish(self) {
        let total = self.cases.len();
        self.finish_with_total(total);
    }

    /// Same as [`TestReport::finish`], with an explicit case total for callers
    /// that track cases outside the report.
    pub fn finish_with_total(self, total: usize) {
        if self.suite_description.is_empty() {
            println!("\n=== {} ===", self.suite_title);
        } else {
            println!("\n=== {} ===\n{}", self.suite_title, self.suite_description);
        }

        let mut failed_lines: Vec<String> = Vec::new();
        for (index, case) in self.cases.iter().enumerate() {
            let line = case.line(index + 1, total);
            println!("{line}");
            if case.failure.is_some() {
                failed_lines.push(line);
            }
        }

        if !failed_lines.is_empty() {
            panic!(
                "\n--- {} / {} tests failed ---\n{}\n",
                failed_lines.len(),
                total,
                failed_lines.join("\n")
            );
        }

        println!("--- {} / {} tests passed ---\n", total, total);
    }
}

/// Old-3 style suite exit: panic once with a summary when any case failed.
///
/// Kept as a thin delegate over [`TestReport`] for suites that collect
/// preformatted `"title || detail"` failure lines themselves.
pub fn report_failures(failures: Vec<String>, total: usize) {
    let mut report = TestReport::new("report_failures (legacy call shape)", "");
    for line in failures {
        match line.split_once(" || ") {
            Some((title, detail)) => report.fail(title.to_string(), detail.to_string()),
            None => report.fail(line, ""),
        }
    }
    report.finish_with_total(total);
}
