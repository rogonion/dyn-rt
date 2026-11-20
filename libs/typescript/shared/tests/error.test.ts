// Data-driven suite for src/error.ts (ADR 0004/0009): JsonError variants and
// their messages (mirrors the Rust display contract).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { JsonError } from "../src/index.js";

interface ErrorCase {
  testTitle?: string;
  make: () => JsonError;
  expectedKind: string;
  expectedMessage: string;
}

const errorCases: ErrorCase[] = [
  {
    testTitle: "InvalidPath",
    make: () => JsonError.invalidPath("$.."),
    expectedKind: "InvalidPath",
    expectedMessage: "invalid JSON path: $..",
  },
  {
    testTitle: "PathNotFound",
    make: () => JsonError.pathNotFound("$.a.b"),
    expectedKind: "PathNotFound",
    expectedMessage: "path not found: $.a.b",
  },
  {
    testTitle: "TypeMismatch",
    make: () => JsonError.typeMismatch("$.a", "array", "string"),
    expectedKind: "TypeMismatch",
    expectedMessage: "type mismatch at $.a: expected array, found string",
  },
  {
    testTitle: "InvalidValue",
    make: () => JsonError.invalidValue("boom"),
    expectedKind: "InvalidValue",
    expectedMessage: "invalid value: boom",
  },
];

test("JsonError cases (src/error.ts)", () => {
  const report = new TestReport("JsonError_cases", "JsonError variants + messages (src/error.ts)");
  for (const testCase of errorCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const err = testCase.make();
    if (err.kind !== testCase.expectedKind) {
      report.failCase(`expected kind ${testCase.expectedKind}, got ${err.kind}`);
    }
    if (err.message !== testCase.expectedMessage) {
      report.failCase(`expected message ${JSON.stringify(testCase.expectedMessage)}, got ${JSON.stringify(err.message)}`);
    }
    if (!(err instanceof Error) || err.name !== "JsonError") {
      report.failCase("JsonError is not an Error with name JsonError");
    }
  }
  report.finish();
});
