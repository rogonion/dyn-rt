// Data-driven test harness (ADR 0004), ported from the Rust
// libs/rust/test-support harness: case tables are plain TypeScript arrays of
// case objects, each suite is one `test(...)` looping over its table, outcomes
// are collected in a TestReport and reported once with the case title and
// expected-vs-got — so the tables double as usage examples and every run
// shows coverage (which cases ran and passed), not just failures.

/**
 * A basic case title from a case's `testTitle` field, based on whether it was
 * set. The case number is added by TestReport when it logs the case.
 */
export function newTestTitle(testTitle: string | undefined): string {
  if (testTitle !== undefined) {
    return `[[[ ${testTitle} ]]]`;
  }
  return "(untitled case)";
}

/** Render a failure detail: strings as-is, everything else as JSON. */
function detailToString(detail: unknown): string {
  if (typeof detail === "string") {
    return detail;
  }
  try {
    const rendered = JSON.stringify(detail);
    return rendered === undefined ? String(detail) : rendered;
  } catch {
    return String(detail);
  }
}

/** One recorded case: its title plus the failure detail, if it failed. */
interface CaseOutcome {
  title: string;
  failure: string | undefined;
}

/**
 * General-purpose suite report (ADR 0004): a root suite title + description,
 * and one log line per case — pass or fail — so every run shows full
 * coverage. By default all cases are evaluated and a single throw at
 * `finish()` reports the failures; with `abortOnFailure()` the first failing
 * case throws immediately instead.
 *
 * Loop idiom inside a suite:
 *
 * ```ts
 * const testData = yieldTestData();
 * const report = new TestReport("suite title", "what this suite covers");
 * for (const testCase of testData) {
 *   report.beginCase(newTestTitle(testCase.testTitle));
 *   // ... on mismatch: report.failCase(`expected ${expected}, got ${got}`);
 * }
 * report.finish();
 * ```
 */
export class TestReport {
  private readonly suiteTitle: string;
  private readonly suiteDescription: string;
  private cases: CaseOutcome[] = [];
  private abortOnFailureFlag = false;

  /**
   * Start a suite report with a root `suiteTitle` and a `suiteDescription`
   * of what the suite is about.
   */
  constructor(suiteTitle: string, suiteDescription: string) {
    this.suiteTitle = suiteTitle;
    this.suiteDescription = suiteDescription;
  }

  /**
   * Abort the suite on the first failing case (throw immediately from
   * `failCase`) instead of collecting failures for the end-of-run throw.
   */
  abortOnFailure(): this {
    this.abortOnFailureFlag = true;
    return this;
  }

  /**
   * Register the case currently being checked. Call once per loop iteration,
   * right after computing the case title; the case is a pass unless
   * `failCase` marks it failed.
   */
  beginCase(title: string): void {
    this.cases.push({ title, failure: undefined });
  }

  /**
   * Mark the most recently begun case as failed with `detail`. With
   * `abortOnFailure()` this throws immediately.
   */
  failCase(detail: unknown): void {
    const last = this.cases[this.cases.length - 1];
    if (last === undefined) {
      throw new Error("failCase called before beginCase");
    }
    last.failure = detailToString(detail);
    if (this.abortOnFailureFlag) {
      const total = this.cases.length;
      throw new Error(
        `\n--- aborting suite on failed test case ---\n${this.line(last, total, total)}\n`,
      );
    }
  }

  /** Record an already-resolved passing case (ad-hoc checks outside the loop). */
  pass(title: string): void {
    this.beginCase(title);
  }

  /** Record an already-resolved failing case (ad-hoc checks outside the loop). */
  fail(title: string, detail: unknown): void {
    this.beginCase(title);
    this.failCase(detail);
  }

  /**
   * Finish the suite: log the suite header and one line per case
   * (`<number>/<total> : <title> : PASS|FAIL`), then throw once with the
   * failure summary when any case failed.
   */
  finish(): void {
    this.finishWithTotal(this.cases.length);
  }

  /** Same as `finish()`, with an explicit case total for callers that track cases outside the report. */
  finishWithTotal(total: number): void {
    if (this.suiteDescription === "") {
      console.log(`\n=== ${this.suiteTitle} ===`);
    } else {
      console.log(`\n=== ${this.suiteTitle} ===\n${this.suiteDescription}`);
    }

    const failedLines: string[] = [];
    for (let index = 0; index < this.cases.length; index++) {
      const testCase = this.cases[index]!;
      const line = this.line(testCase, index + 1, total);
      console.log(line);
      if (testCase.failure !== undefined) {
        failedLines.push(line);
      }
    }

    if (failedLines.length > 0) {
      throw new Error(
        `\n--- ${failedLines.length} / ${total} tests failed ---\n${failedLines.join("\n")}\n`,
      );
    }

    console.log(`\n--- ${total} / ${total} tests passed ---`);
  }

  /** The per-case log line: `<number>/<total> : <title> : <outcome>`. Failures additionally carry their detail. */
  private line(testCase: CaseOutcome, number: number, total: number): string {
    if (testCase.failure !== undefined) {
      return `Test case -> ${number}/${total} : ${testCase.title} || FAIL || ${testCase.failure}`;
    }
    return `Test case -> ${number}/${total} : ${testCase.title} || PASS`;
  }
}
