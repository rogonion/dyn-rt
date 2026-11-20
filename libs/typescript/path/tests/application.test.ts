// Application-layer suites for path (ADR 0009): the parsePath and
// extractSegment use cases (port of libs/rust/path/tests/application.rs).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, descentSegment, extractSegment, indexSegment, keySegment, parsePath, rootSegment, sliceSegment, unionSegment, wildcardSegment } from "../src/index.js";
import { astEquals, type ExtractCase, type ParseCase } from "./common.js";

test("parse_path_cases (src/application.ts)", () => {
  const testData = yieldParseData();
  const tokenizer = new RegexPathTokenizer();
  const report = new TestReport("parse_path_cases", "Application-layer suites for path (ADR 0009): the parsePath and extractSegment use cases (src/application.ts)");

  for (const testCase of testData) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const result = parsePath(tokenizer, testCase.source);
    if (!astEquals(result, testCase.expected)) {
      report.failCase(`parse mismatch for path ${testCase.source} || expected ${JSON.stringify(testCase.expected)} ; got ${JSON.stringify(result)}`);
    }
  }

  report.finish();
});

function yieldParseData(): ParseCase[] {
  return [
    {
      source: "$.store.book[0].title",
      expected: [descentSegment(rootSegment(), keySegment("store"), keySegment("book"), indexSegment(0), keySegment("title"))],
      testTitle: "standard dot and index path",
    },
    {
      source: "$.store..book[0].title",
      expected: [descentSegment(rootSegment(), keySegment("store")), descentSegment(keySegment("book"), indexSegment(0), keySegment("title"))],
      testTitle: "basic path with recursive descent",
    },
    {
      source: "$..store..book..*",
      expected: [descentSegment(rootSegment()), descentSegment(keySegment("store")), descentSegment(keySegment("book")), descentSegment(wildcardSegment())],
      testTitle: "multiple recursive descents ending in wildcard",
    },
    {
      source: "$['a..b']..['c..d']",
      expected: [descentSegment(rootSegment(), keySegment("a..b")), descentSegment(keySegment("c..d"))],
      testTitle: "string literals containing .. inside brackets",
    },
    {
      source: "$['report-data']..['total.sum..()&^']..store.bicycle['item-code']",
      expected: [
        descentSegment(rootSegment(), keySegment("report-data")),
        descentSegment(keySegment("total.sum..()&^")),
        descentSegment(keySegment("store"), keySegment("bicycle"), keySegment("item-code")),
      ],
      testTitle: "complex symbols with .. inside bracket quotes",
    },
    {
      source: "$['report-data']..['total.sum']..store.bicycle['item-code']",
      expected: [
        descentSegment(rootSegment(), keySegment("report-data")),
        descentSegment(keySegment("total.sum")),
        descentSegment(keySegment("store"), keySegment("bicycle"), keySegment("item-code")),
      ],
      testTitle: "multiple recursive descents with hyphenated keys",
    },
    {
      source: "$.transactions[1:5:2].amount",
      expected: [descentSegment(rootSegment(), keySegment("transactions"), sliceSegment(1, 5, 2), keySegment("amount"))],
      testTitle: "slice selector",
    },
    {
      source: "$.transactions[1:-5].amount",
      expected: [descentSegment(rootSegment(), keySegment("transactions"), sliceSegment(1, -5), keySegment("amount"))],
      testTitle: "slice selector with negative end",
    },
    {
      source: "$.data.user.preferences['theme-settings',\"font-size\",-3]",
      expected: [
        descentSegment(
          rootSegment(),
          keySegment("data"),
          keySegment("user"),
          keySegment("preferences"),
          unionSegment("Mixed", [keySegment("theme-settings"), keySegment("font-size"), indexSegment(-3)]),
        ),
      ],
      testTitle: "union selector with mixed member types",
    },
    {
      source: "$[1,3,5]",
      expected: [descentSegment(rootSegment(), unionSegment("Positional", [indexSegment(1), indexSegment(3), indexSegment(5)]))],
      testTitle: "union of indices",
    },
    {
      source: "$..['1st_category'].name",
      expected: [descentSegment(rootSegment()), descentSegment(keySegment("1st_category"), keySegment("name"))],
      testTitle: "recursive descent into bracketed numeric-leading key",
    },
    {
      source: "$.products['item-details..'].dimensions[0].width.dimensions[-2][3].width",
      expected: [
        descentSegment(
          rootSegment(),
          keySegment("products"),
          keySegment("item-details.."),
          keySegment("dimensions"),
          indexSegment(0),
          keySegment("width"),
          keySegment("dimensions"),
          indexSegment(-2),
          indexSegment(3),
          keySegment("width"),
        ),
      ],
      testTitle: "dots inside property key plus chained array indices",
    },
    {
      source: "$['user info']['address.wind'][1].street",
      expected: [descentSegment(rootSegment(), keySegment("user info"), keySegment("address.wind"), indexSegment(1), keySegment("street"))],
      testTitle: "spaces and dots in quoted keys",
    },
    {
      source: "$.data[*].price",
      expected: [descentSegment(rootSegment(), keySegment("data"), wildcardSegment(), keySegment("price"))],
      testTitle: "wildcard member",
    },
    {
      source: "$.store.bicycle['item-code']",
      expected: [descentSegment(rootSegment(), keySegment("store"), keySegment("bicycle"), keySegment("item-code"))],
      testTitle: "bracketed property key",
    },
    {
      source: "$..author",
      expected: [descentSegment(rootSegment()), descentSegment(keySegment("author"))],
      testTitle: "single recursive descent",
    },
    {
      source: "$['store\\'s..item']..id",
      expected: [descentSegment(rootSegment(), keySegment("store\\'s..item")), descentSegment(keySegment("id"))],
      testTitle: "escaped single quote inside key containing ..",
    },
    {
      source: '$["user\\"name..data"]..title',
      expected: [descentSegment(rootSegment(), keySegment('user\\"name..data')), descentSegment(keySegment("title"))],
      testTitle: "escaped double quote inside key containing ..",
    },
    {
      source: "$['']..author",
      expected: [descentSegment(rootSegment(), keySegment("")), descentSegment(keySegment("author"))],
      testTitle: "empty bracketed string key",
    },
    {
      source: "$['title_\"main\"..part']..id",
      expected: [descentSegment(rootSegment(), keySegment('title_"main"..part')), descentSegment(keySegment("id"))],
      testTitle: "double quotes inside single-quoted key with ..",
    },
    {
      source: "$['report-data']..['total.sum']",
      expected: [descentSegment(rootSegment(), keySegment("report-data")), descentSegment(keySegment("total.sum"))],
      testTitle: "two recursive descent groups with hyphenated keys (go/js variant)",
    },
    {
      source: "$.transactions[1:5:2].*",
      expected: [descentSegment(rootSegment(), keySegment("transactions"), sliceSegment(1, 5, 2), wildcardSegment())],
      testTitle: "slice followed by wildcard (js variant)",
    },
    {
      source: "$.data.user.preferences['theme-settings','font-size']",
      expected: [descentSegment(rootSegment(), keySegment("data"), keySegment("user"), keySegment("preferences"), unionSegment("Named", [keySegment("theme-settings"), keySegment("font-size")]))],
      testTitle: "union of two named keys (go/js variant)",
    },
    {
      source: "$.User['ID', 'Name']",
      expected: [descentSegment(rootSegment(), keySegment("User"), unionSegment("Named", [keySegment("ID"), keySegment("Name")]))],
      testTitle: "union members after a comma+space are still unquoted",
    },
    {
      source: "$['a', *]",
      expected: [descentSegment(rootSegment(), unionSegment("Mixed", [keySegment("a"), wildcardSegment()]))],
      testTitle: "wildcard member after a comma+space in a union",
    },
    {
      source: "$.products['item-details'].dimensions[0].width",
      expected: [descentSegment(rootSegment(), keySegment("products"), keySegment("item-details"), keySegment("dimensions"), indexSegment(0), keySegment("width"))],
      testTitle: "hyphenated key without inner dots (go/js variant)",
    },
  ];
}

test("extract_segment_cases (src/application.ts)", () => {
  const testData = yieldExtractData();
  const tokenizer = new RegexPathTokenizer();
  const report = new TestReport("extract_segment_cases", "extractSegment use case (src/application.ts)");

  for (const testCase of testData) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const result = extractSegment(tokenizer, testCase.source);
    if (!astEquals(result, testCase.expected)) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)} ; got ${JSON.stringify(result)}`);
    }
  }

  report.finish();
});

function yieldExtractData(): ExtractCase[] {
  return [
    { source: "$", expected: rootSegment(), testTitle: "standalone root selector" },
    { source: "store", expected: keySegment("store"), testTitle: "standalone property name" },
    { source: "*", expected: wildcardSegment(), testTitle: "standalone wildcard selector" },
    {
      source: "'a..b'",
      expected: keySegment("a..b"),
      testTitle: "quoted bracket key with inner dots",
    },
    {
      source: "'report-data'",
      expected: keySegment("report-data"),
      testTitle: "hyphenated quoted key",
    },
    {
      source: "'total.sum..()&^'",
      expected: keySegment("total.sum..()&^"),
      testTitle: "complex symbols and dots inside single quotes",
    },
    {
      source: "1:5:2",
      expected: sliceSegment(1, 5, 2),
      testTitle: "slice start:end:step",
    },
    { source: "1:5", expected: sliceSegment(1, 5), testTitle: "slice start:end" },
    {
      source: "::-2",
      expected: sliceSegment(undefined, undefined, -2),
      testTitle: "slice step only",
    },
    { source: ":4:-2", expected: sliceSegment(undefined, 4, -2), testTitle: "slice end:step" },
    {
      source: `'theme-settings',"font-size",-3, -1::2`,
      expected: unionSegment("Mixed", [keySegment("theme-settings"), keySegment("font-size"), indexSegment(-3), sliceSegment(-1, undefined, 2)]),
      testTitle: "mixed union selector with key, key, index, slice members",
    },
    {
      source: "'user info'",
      expected: keySegment("user info"),
      testTitle: "quoted key with space",
    },
    {
      source: "'store\\'s..item'",
      expected: keySegment("store\\'s..item"),
      testTitle: "escaped quote bracket key",
    },
    {
      source: '"user\\"name..data"',
      expected: keySegment('user\\"name..data'),
      testTitle: "escaped double quote bracket key",
    },
    { source: "''", expected: keySegment(""), testTitle: "empty string bracket key" },
    {
      source: `'title_"main"..part'`,
      expected: keySegment('title_"main"..part'),
      testTitle: "mixed quote bracket key",
    },
  ];
}
