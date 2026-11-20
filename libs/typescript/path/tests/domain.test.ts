// Domain-layer suite for path (ADR 0009): rendering of the AST back to a
// path string (round-trip), built with the segment constructor helpers so
// the table reads like the paths themselves (port of
// libs/rust/path/tests/domain.rs).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { descentSegment, indexSegment, keySegment, renderSegments, rootSegment, sliceSegment, unionSegment, wildcardSegment } from "../src/index.js";
import type { RenderCase } from "./common.js";

test("ast_render_roundtrip (src/domain.ts)", () => {
  const testData = yieldRenderData();
  const report = new TestReport("ast_render_roundtrip", "Domain-layer suite for path (ADR 0009): rendering of the AST (src/domain.ts)");

  for (const testCase of testData) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const rendered = renderSegments(testCase.segments);
    if (rendered !== testCase.roundtrip) {
      report.failCase(`expected ${JSON.stringify(testCase.roundtrip)}, got ${JSON.stringify(rendered)}`);
    }
  }

  report.finish();
});

function yieldRenderData(): RenderCase[] {
  return [
    {
      segments: [descentSegment(rootSegment(), keySegment("store"), keySegment("book"), indexSegment(0), keySegment("title"))],
      roundtrip: "$.store.book[0].title",
      testTitle: "standard dot and index path",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("store")), descentSegment(keySegment("book"), indexSegment(0), keySegment("title"))],
      roundtrip: "$.store..book[0].title",
      testTitle: "basic path with recursive descent",
    },
    {
      segments: [descentSegment(rootSegment()), descentSegment(keySegment("store")), descentSegment(keySegment("book")), descentSegment(wildcardSegment())],
      roundtrip: "$..store..book..*",
      testTitle: "multiple recursive descents ending in wildcard",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("a..b")), descentSegment(keySegment("c..d"))],
      roundtrip: "$['a..b']..['c..d']",
      testTitle: "string literals containing .. inside brackets",
    },
    {
      segments: [
        descentSegment(rootSegment(), keySegment("report-data")),
        descentSegment(keySegment("total.sum..()&^")),
        descentSegment(keySegment("store"), keySegment("bicycle"), keySegment("item-code")),
      ],
      roundtrip: "$['report-data']..['total.sum..()&^']..store.bicycle['item-code']",
      testTitle: "complex symbols with .. inside bracket quotes",
    },
    {
      segments: [
        descentSegment(rootSegment(), keySegment("report-data")),
        descentSegment(keySegment("total.sum")),
        descentSegment(keySegment("store"), keySegment("bicycle"), keySegment("item-code")),
      ],
      roundtrip: "$['report-data']..['total.sum']..store.bicycle['item-code']",
      testTitle: "multiple recursive descents with hyphenated keys",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("transactions"), sliceSegment(1, 5, 2), keySegment("amount"))],
      roundtrip: "$.transactions[1:5:2].amount",
      testTitle: "slice selector",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("transactions"), sliceSegment(1, -5), keySegment("amount"))],
      roundtrip: "$.transactions[1:-5].amount",
      testTitle: "slice selector with negative end",
    },
    {
      segments: [
        descentSegment(
          rootSegment(),
          keySegment("data"),
          keySegment("user"),
          keySegment("preferences"),
          unionSegment("Mixed", [keySegment("theme-settings"), keySegment("font-size"), indexSegment(-3)]),
        ),
      ],
      roundtrip: "$.data.user.preferences['theme-settings','font-size',-3]",
      testTitle: "union selector with mixed member types",
    },
    {
      segments: [descentSegment(rootSegment(), unionSegment("Positional", [indexSegment(1), indexSegment(3), indexSegment(5)]))],
      roundtrip: "$[1,3,5]",
      testTitle: "union of indices",
    },
    {
      segments: [descentSegment(rootSegment()), descentSegment(keySegment("1st_category"), keySegment("name"))],
      roundtrip: "$..['1st_category'].name",
      testTitle: "recursive descent into bracketed numeric-leading key",
    },
    {
      segments: [
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
      roundtrip: "$.products['item-details..'].dimensions[0].width.dimensions[-2][3].width",
      testTitle: "dots inside property key plus chained array indices",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("user info"), keySegment("address.wind"), indexSegment(1), keySegment("street"))],
      roundtrip: "$['user info']['address.wind'][1].street",
      testTitle: "spaces and dots in quoted keys",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("data"), wildcardSegment(), keySegment("price"))],
      roundtrip: "$.data.*.price",
      testTitle: "wildcard member",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("store"), keySegment("bicycle"), keySegment("item-code"))],
      roundtrip: "$.store.bicycle['item-code']",
      testTitle: "bracketed property key",
    },
    {
      segments: [descentSegment(rootSegment()), descentSegment(keySegment("author"))],
      roundtrip: "$..author",
      testTitle: "single recursive descent",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("store\\'s..item")), descentSegment(keySegment("id"))],
      roundtrip: "$['store\\'s..item']..id",
      testTitle: "escaped single quote inside key containing ..",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment('user\\"name..data')), descentSegment(keySegment("title"))],
      roundtrip: "$['user\\\"name..data']..title",
      testTitle: "escaped double quote inside key containing ..",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("")), descentSegment(keySegment("author"))],
      roundtrip: "$['']..author",
      testTitle: "empty bracketed string key",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment('title_"main"..part')), descentSegment(keySegment("id"))],
      roundtrip: "$['title_\"main\"..part']..id",
      testTitle: "double quotes inside single-quoted key with ..",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("report-data")), descentSegment(keySegment("total.sum"))],
      roundtrip: "$['report-data']..['total.sum']",
      testTitle: "two recursive descent groups with hyphenated keys (go/js variant)",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("transactions"), sliceSegment(1, 5, 2), wildcardSegment())],
      roundtrip: "$.transactions[1:5:2].*",
      testTitle: "slice followed by wildcard (js variant)",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("data"), keySegment("user"), keySegment("preferences"), unionSegment("Named", [keySegment("theme-settings"), keySegment("font-size")]))],
      roundtrip: "$.data.user.preferences['theme-settings','font-size']",
      testTitle: "union of two named keys (go/js variant)",
    },
    {
      segments: [descentSegment(rootSegment(), keySegment("products"), keySegment("item-details"), keySegment("dimensions"), indexSegment(0), keySegment("width"))],
      roundtrip: "$.products['item-details'].dimensions[0].width",
      testTitle: "hyphenated key without inner dots (go/js variant)",
    },
  ];
}
