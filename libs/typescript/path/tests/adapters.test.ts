// Adapter suite for src/adapters.ts's RegexPathTokenizer: the three split
// steps of the tokenizer port (port of
// libs/rust/path/tests/adapters/regex_tokenizer.rs).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer } from "../src/index.js";
import type { SplitCase } from "./common.js";

function runSplitSuite(suiteTitle: string, description: string, testData: SplitCase[], split: (tokenizer: RegexPathTokenizer, source: string) => string[]): void {
  const tokenizer = new RegexPathTokenizer();
  const report = new TestReport(suiteTitle, description);
  for (const testCase of testData) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const result = split(tokenizer, testCase.source);
    if (JSON.stringify(result) !== JSON.stringify(testCase.expected)) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)} ; got ${JSON.stringify(result)}`);
    }
  }
  report.finish();
}

test("split_recursive_descent (src/adapters.ts)", () => {
  runSplitSuite("split_recursive_descent", "Adapter suite for src/adapters.ts's RegexPathTokenizer: splitRecursiveDescent step", yieldSplitRecursiveDescent(), (t, s) => t.splitRecursiveDescent(s));
});

function yieldSplitRecursiveDescent(): SplitCase[] {
  return [
    {
      source: "$.store..book[0].title",
      expected: ["$.store", "book[0].title"],
      testTitle: "basic path with some recursive descent",
    },
    {
      source: "$..store..book..*",
      expected: ["$", "store", "book", "*"],
      testTitle: "multiple recursive descents starting with $..",
    },
    {
      source: "$['a..b']..['c..d']",
      expected: ["$['a..b']", "['c..d']"],
      testTitle: "string literals containing .. inside brackets",
    },
    {
      source: "$['report-data']..['total.sum..()&^']..store.bicycle['item-code']",
      expected: ["$['report-data']", "['total.sum..()&^']", "store.bicycle['item-code']"],
      testTitle: "complex symbols with .. inside bracket quotes",
    },
    {
      source: "$['report-data']..['total.sum']..store.bicycle['item-code']",
      expected: ["$['report-data']", "['total.sum']", "store.bicycle['item-code']"],
      testTitle: "multiple recursive descents with hyphenated keys",
    },
    {
      source: "$.transactions[1:5:2].amount",
      expected: ["$.transactions[1:5:2].amount"],
      testTitle: "slice selector",
    },
    {
      source: "$.data.user.preferences['theme-settings',\"font-size\",3]",
      expected: ["$.data.user.preferences['theme-settings',\"font-size\",3]"],
      testTitle: "union selector with mixed types",
    },
    {
      source: "$..['1st_category'].name",
      expected: ["$", "['1st_category'].name"],
      testTitle: "recursive descent into bracketed numeric key",
    },
    {
      source: "$.products['item-details..'].dimensions[0].width.dimensions[2][3].width",
      expected: ["$.products['item-details..'].dimensions[0].width.dimensions[2][3].width"],
      testTitle: "dots inside property key + chained array indices",
    },
    {
      source: "$['user info']['address.wind'][1].street",
      expected: ["$['user info']['address.wind'][1].street"],
      testTitle: "spaces and dots in quoted key",
    },
    {
      source: "$.data[*].price",
      expected: ["$.data[*].price"],
      testTitle: "wildcard array match",
    },
    {
      source: "$.store.bicycle['item-code']",
      expected: ["$.store.bicycle['item-code']"],
      testTitle: "bracketed property key",
    },
    { source: "$..author", expected: ["$", "author"], testTitle: "single recursive descent" },
    {
      source: "$.store.book[0].title",
      expected: ["$.store.book[0].title"],
      testTitle: "standard dot and index path",
    },
    {
      source: "$['store\\'s..item']..id",
      expected: ["$['store\\'s..item']", "id"],
      testTitle: "escaped single quote inside key containing ..",
    },
    {
      source: '$["user\\"name..data"]..title',
      expected: ['$["user\\"name..data"]', "title"],
      testTitle: "escaped double quote inside key containing ..",
    },
    {
      source: "$['']..author",
      expected: ["$['']", "author"],
      testTitle: "empty bracketed string key",
    },
    {
      source: "$['title_\"main\"..part']..id",
      expected: ["$['title_\"main\"..part']", "id"],
      testTitle: "mixed quotes inside single-quoted key with ..",
    },
  ];
}

test("split_dot_notation (src/adapters.ts)", () => {
  runSplitSuite("split_dot_notation", "Adapter suite for src/adapters.ts's RegexPathTokenizer: splitDotNotation step", yieldSplitDotNotation(), (t, s) => t.splitDotNotation(s));
});

function yieldSplitDotNotation(): SplitCase[] {
  return [
    { source: "$.store", expected: ["$", "store"], testTitle: "root and simple key" },
    {
      source: "book[0].title",
      expected: ["book[0]", "title"],
      testTitle: "bracket index attached to key + property title",
    },
    { source: "$", expected: ["$"], testTitle: "standalone root selector" },
    { source: "store", expected: ["store"], testTitle: "standalone property name" },
    { source: "*", expected: ["*"], testTitle: "standalone wildcard selector" },
    {
      source: "$['a..b']",
      expected: ["$['a..b']"],
      testTitle: "root with quoted bracket key containing inner dots",
    },
    {
      source: "['c..d']",
      expected: ["['c..d']"],
      testTitle: "standalone quoted bracket key with inner dots",
    },
    {
      source: "$['report-data']",
      expected: ["$['report-data']"],
      testTitle: "root with hyphenated quoted key",
    },
    {
      source: "['total.sum..()&^']",
      expected: ["['total.sum..()&^']"],
      testTitle: "complex symbols and dots inside single quotes",
    },
    {
      source: "store.bicycle['item-code']",
      expected: ["store", "bicycle['item-code']"],
      testTitle: "chained dot property with bracketed property key",
    },
    {
      source: "$.transactions[1:5:2].amount",
      expected: ["$", "transactions[1:5:2]", "amount"],
      testTitle: "slice selector attached to property name",
    },
    {
      source: "$.data.user.preferences['theme-settings',\"font-size\",3]",
      expected: ["$", "data", "user", "preferences['theme-settings',\"font-size\",3]"],
      testTitle: "deeply nested property path with mixed union selector",
    },
    {
      source: "['1st_category'].name",
      expected: ["['1st_category']", "name"],
      testTitle: "bracketed category identifier followed by dot property",
    },
    {
      source: "$.products['item-details..'].dimensions[0].width.dimensions[2][3].width",
      expected: ["$", "products['item-details..']", "dimensions[0]", "width", "dimensions[2][3]", "width"],
      testTitle: "complex path with dots in quotes, array indices, chained brackets",
    },
    {
      source: "$['user info']['address.wind'][1].street",
      expected: ["$['user info']['address.wind'][1]", "street"],
      testTitle: "spaces and dots inside quoted brackets with array indexing",
    },
    {
      source: "$.data[*].price",
      expected: ["$", "data[*]", "price"],
      testTitle: "wildcard array index attached to property",
    },
    { source: "author", expected: ["author"], testTitle: "single simple identifier" },
    {
      source: "$['store\\'s..item']",
      expected: ["$['store\\'s..item']"],
      testTitle: "escaped single quote and dots inside bracket key",
    },
    {
      source: '$["user\\"name..data"]',
      expected: ['$["user\\"name..data"]'],
      testTitle: "escaped double quote and dots inside bracket key",
    },
    { source: "$['']", expected: ["$['']"], testTitle: "empty single-quoted string bracket key" },
    {
      source: "$['title_\"main\"..part']",
      expected: ["$['title_\"main\"..part']"],
      testTitle: "mixed double quotes and dots inside single-quoted bracket key",
    },
  ];
}

test("split_brackets (src/adapters.ts)", () => {
  runSplitSuite("split_brackets", "Adapter suite for src/adapters.ts's RegexPathTokenizer: splitBrackets step", yieldSplitBrackets(), (t, s) => t.splitBrackets(s));
});

function yieldSplitBrackets(): SplitCase[] {
  return [
    { source: "$", expected: ["$"], testTitle: "standalone root selector" },
    { source: "store", expected: ["store"], testTitle: "standalone property name" },
    { source: "*", expected: ["*"], testTitle: "standalone wildcard selector" },
    {
      source: "$['a..b']",
      expected: ["$", "'a..b'"],
      testTitle: "root split from quoted bracket key",
    },
    {
      source: "['c..d']",
      expected: ["'c..d'"],
      testTitle: "standalone quoted bracket key with inner dots",
    },
    {
      source: "$['report-data']",
      expected: ["$", "'report-data'"],
      testTitle: "root split from hyphenated quoted key",
    },
    {
      source: "['total.sum..()&^']",
      expected: ["'total.sum..()&^'"],
      testTitle: "complex symbols and dots inside single quotes",
    },
    {
      source: "bicycle['item-code']",
      expected: ["bicycle", "'item-code'"],
      testTitle: "chained dot property with bracketed property key",
    },
    {
      source: "transactions[1:5:2]",
      expected: ["transactions", "1:5:2"],
      testTitle: "slice selector split from property name",
    },
    {
      source: "preferences['theme-settings',\"font-size\",3]",
      expected: ["preferences", "'theme-settings',\"font-size\",3"],
      testTitle: "mixed union selector split from property name",
    },
    {
      source: "['1st_category']",
      expected: ["'1st_category'"],
      testTitle: "bracketed category identifier",
    },
    {
      source: "products['item-details..']",
      expected: ["products", "'item-details..'"],
      testTitle: "property key containing dots",
    },
    { source: "dimensions[0]", expected: ["dimensions", "0"], testTitle: "single array index" },
    {
      source: "dimensions[2][3]",
      expected: ["dimensions", "2", "3"],
      testTitle: "chained array indices",
    },
    {
      source: "$['user info']['address.wind'][1]",
      expected: ["$", "'user info'", "'address.wind'", "1"],
      testTitle: "chained bracket keys into individual atomic brackets",
    },
    {
      source: "data[*]",
      expected: ["data", "*"],
      testTitle: "wildcard array index split from property",
    },
    { source: "author", expected: ["author"], testTitle: "single simple identifier" },
    {
      source: "$['store\\'s..item']",
      expected: ["$", "'store\\'s..item'"],
      testTitle: "root split from escaped quote bracket key",
    },
    {
      source: '$["user\\"name..data"]',
      expected: ["$", '"user\\"name..data"'],
      testTitle: "root split from escaped double quote bracket key",
    },
    {
      source: "$['']",
      expected: ["$", "''"],
      testTitle: "root split from empty string bracket key",
    },
    {
      source: "$['title_\"main\"..part']",
      expected: ["$", "'title_\"main\"..part'"],
      testTitle: "root split from mixed quote bracket key",
    },
  ];
}
