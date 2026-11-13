// Smoke suite for the facade (ADR 0004/0009): proves the packages/typescript
// entrypoint re-exports the libs (src/index.ts).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { shared } from "../src/index.js";

interface SmokeCase {
  testTitle?: string;
  run: () => unknown;
  expected: unknown;
}

const smokeCases: SmokeCase[] = [
  {
    testTitle: "facade re-exports shared.typeName",
    run: () => shared.typeName({ a: 1 }),
    expected: "object",
  },
  {
    testTitle: "facade re-exports shared.kindOf",
    run: () => shared.kindOf(10n),
    expected: "bigint",
  },
  {
    testTitle: "facade re-exports shared.JsonError",
    run: () => shared.JsonError.pathNotFound("$.x").message,
    expected: "path not found: $.x",
  },
];

test("facade smoke cases (src/index.ts)", () => {
  const report = new TestReport("facade_smoke_cases", "facade re-exports libs (src/index.ts)");
  for (const testCase of smokeCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = testCase.run();
    if (got !== testCase.expected) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
    }
  }
  report.finish();
});
