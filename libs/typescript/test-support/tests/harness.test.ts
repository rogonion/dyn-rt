// Data-driven suite for the test harness itself (ADR 0004/0009):
// tests src/harness.ts (TestReport, newTestTitle).

import { test } from "vitest";
import { TestReport, newTestTitle } from "../src/index.js";

interface TitleCase {
  testTitle?: string;
  titleField: string | undefined;
  expected: string;
}

const titleCases: TitleCase[] = [
  {
    testTitle: "newTestTitle renders a set title",
    titleField: "a case title",
    expected: "[[[ a case title ]]]",
  },
  {
    testTitle: "newTestTitle renders an unset title",
    titleField: undefined,
    expected: "(untitled case)",
  },
];

test("newTestTitle cases (src/harness.ts)", () => {
  const report = new TestReport(
    "newTestTitle_cases",
    "newTestTitle title rendering (src/harness.ts)",
  );
  for (const testCase of titleCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = newTestTitle(testCase.titleField);
    if (got !== testCase.expected) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
    }
  }
  report.finish();
});

interface ReportCase {
  testTitle?: string;
  // when set, the case is failed with this detail; when undefined the case passes
  failWith: string | undefined;
  expectFinishThrow: boolean;
}

const reportCases: ReportCase[] = [
  {
    testTitle: "all-passing suite finishes without throwing",
    failWith: undefined,
    expectFinishThrow: false,
  },
  {
    testTitle: "a failed case makes finish() throw",
    failWith: "expected 1, got 2",
    expectFinishThrow: true,
  },
];

test("TestReport collect-and-finish behavior (src/harness.ts)", () => {
  const report = new TestReport(
    "TestReport_cases",
    "TestReport pass/fail collection and finish() throw behavior (src/harness.ts)",
  );
  for (const testCase of reportCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    let threw = false;
    const inner = new TestReport("inner", "");
    inner.beginCase("inner case");
    if (testCase.failWith !== undefined) {
      inner.failCase(testCase.failWith);
    }
    try {
      inner.finish();
    } catch {
      threw = true;
    }
    if (threw !== testCase.expectFinishThrow) {
      report.failCase(
        `expected finish() to throw=${testCase.expectFinishThrow}, got threw=${threw}`,
      );
    }
  }
  report.finish();
});

test("TestReport.abortOnFailure throws from failCase immediately (src/harness.ts)", () => {
  const report = new TestReport(
    "abort_on_failure_case",
    "abortOnFailure mode throws on the first failing case (src/harness.ts)",
  );
  report.beginCase("abort-on-failure aborts at failCase");
  let threw = false;
  const inner = new TestReport("inner", "").abortOnFailure();
  inner.beginCase("doomed case");
  try {
    inner.failCase("boom");
  } catch {
    threw = true;
  }
  if (!threw) {
    report.failCase("expected failCase to throw in abortOnFailure mode");
  }
  report.finish();
});
