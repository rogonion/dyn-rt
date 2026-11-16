// Adapters for the path module (port of libs/rust/path/src/adapters.rs):
// the regex tokenizer. Each regex is a named, single-purpose pattern applied
// in one explicit step of the parse pipeline (ADR 0005) — ported from
// dyn-rt-rs-old-3, the logic reference for the parser. (The Rust crate keeps
// `regex` confined to its adapters dir; in TS, RegExp is stdlib, and this
// file is still the only place the path module uses it.)

import type { PathTokenizer, SliceSelector } from "./domain.js";

// Quoted-string fragments (shared by several patterns). Bracket contents
// accept single- and double-quoted strings; the union splitter and quote
// extractor additionally accept backtick-quoted strings.
const QUOTED_SINGLE = String.raw`'[^'\\]*(?:\\.[^'\\]*)*'`;
const QUOTED_DOUBLE = String.raw`"[^"\\]*(?:\\.[^"\\]*)*"`;
// (a normal string literal, not String.raw: template literals cannot
// contain an unescaped backtick)
const QUOTED_BACKTICK = "`[^`\\\\]*(?:\\\\.[^`\\\\]*)*`";
const BRACKET_CONTENT = String.raw`(?:${QUOTED_SINGLE}|${QUOTED_DOUBLE}|[^\]])+`;

/** Matches bracketed selector expressions `[...]` (ignoring internal dots)
 * OR the standalone recursive descent operator `..`.
 *
 * Used in step 1 (`splitRecursiveDescent`). */
export const RECURSIVE_DESCENT_SPLIT_REGEX = new RegExp(
  String.raw`\[${BRACKET_CONTENT}\]|(?<descent>\.\.)`,
  "g",
);

/** Matches bracketed selector expressions `[...]` (ignoring internal dots)
 * OR the standalone dot notation operator `.`.
 *
 * Used in step 2 (`splitDotNotation`). */
export const MEMBER_DOT_NOTATION_REGEX = new RegExp(
  String.raw`\[${BRACKET_CONTENT}\]|(?<dot>\.)`,
  "g",
);

/** Splits a part into bracket contents (`ibracket`) and non-bracket text
 * (`obracket`). Used in step 3 (`splitBrackets`). */
export const BRACKET_SPLIT_REGEX = new RegExp(
  String.raw`\[(?<ibracket>${BRACKET_CONTENT})\]|(?<obracket>[^\[]+)`,
  "g",
);

/** Matches a positional index: an optionally negative integer, anchored. */
export const POSITIONAL_INDEX_REGEX = /^(?<position>-?\d+)$/;

/** Matches a linear collection slice `start:end:step` (all parts optional,
 * negatives allowed), e.g. `:`, `1:5`, `::2`, `-1:-5:-1`. */
export const SLICE_SELECTOR_REGEX = /(?<start>-?\d*):(?<end>-?\d*)(?::(?<step>-?\d*))?/;

/** Matches the union splitter `,` while ignoring commas inside quotes.
 * Used in step 4a (`splitUnion`). */
export const UNION_SPLIT_REGEX = new RegExp(
  String.raw`${QUOTED_SINGLE}|${QUOTED_DOUBLE}|${QUOTED_BACKTICK}|(?<comma>,)`,
  "g",
);

/** Matches a fully quoted string and captures the unquoted content per quote
 * style: single, double, backtick. */
export const QUOTE_EXTRACTOR_REGEX = new RegExp(
  "^(?:'(?<single>[^'\\\\]*(?:\\\\.[^'\\\\]*)*)'" +
    '|"(?<double>[^"\\\\]*(?:\\\\.[^"\\\\]*)*)"' +
    "|`(?<backtick>[^`\\\\]*(?:\\\\.[^`\\\\]*)*)`" +
    ")$",
);

/** Parse an optional signed-integer capture: `""` and bare `"-"` are not
 * numbers (mirrors Rust's `parse::<isize>()` failing on them). */
function parseOptInt(text: string | undefined): number | undefined {
  if (text === undefined || text === "" || text === "-") return undefined;
  const value = Number(text);
  return Number.isNaN(value) ? undefined : value;
}

/** The default PathTokenizer: the named regexes above applied in the
 * explicit step order of the parse pipeline. */
export class RegexPathTokenizer implements PathTokenizer {
  splitRecursiveDescent(path: string): string[] {
    const segments: string[] = [];
    let lastDescentEndIndex = 0;

    for (const match of path.matchAll(RECURSIVE_DESCENT_SPLIT_REGEX)) {
      const descent = match.groups?.descent;
      if (descent !== undefined) {
        segments.push(path.slice(lastDescentEndIndex, match.index));
        lastDescentEndIndex = match.index + match[0].length;
      }
    }

    // append the final segment
    segments.push(path.slice(lastDescentEndIndex));
    return segments;
  }

  splitDotNotation(part: string): string[] {
    const segments: string[] = [];
    let lastDotEndIndex = 0;

    for (const match of part.matchAll(MEMBER_DOT_NOTATION_REGEX)) {
      const dot = match.groups?.dot;
      if (dot !== undefined) {
        segments.push(part.slice(lastDotEndIndex, match.index));
        lastDotEndIndex = match.index + match[0].length;
      }
    }

    // append the final segment
    segments.push(part.slice(lastDotEndIndex));
    return segments;
  }

  splitBrackets(part: string): string[] {
    const segments: string[] = [];

    for (const match of part.matchAll(BRACKET_SPLIT_REGEX)) {
      const inBracket = match.groups?.ibracket;
      if (inBracket !== undefined) {
        segments.push(inBracket);
        continue;
      }
      const outBracket = match.groups?.obracket;
      if (outBracket !== undefined) {
        const trimmed = outBracket.trim();
        if (trimmed !== "") {
          segments.push(trimmed);
        }
      }
    }

    return segments;
  }

  splitUnion(content: string): string[] {
    const parts: string[] = [];
    let lastUnionEndIndex = 0;

    for (const match of content.matchAll(UNION_SPLIT_REGEX)) {
      const comma = match.groups?.comma;
      if (comma !== undefined) {
        parts.push(content.slice(lastUnionEndIndex, match.index));
        lastUnionEndIndex = match.index + match[0].length;
      }
    }

    // append the final part
    parts.push(content.slice(lastUnionEndIndex));
    return parts;
  }

  extractQuoted(token: string): string | undefined {
    const match = QUOTE_EXTRACTOR_REGEX.exec(token);
    if (match === null) return undefined;
    const groups = match.groups ?? {};
    if (groups.single !== undefined) return groups.single;
    if (groups.double !== undefined) return groups.double;
    if (groups.backtick !== undefined) return groups.backtick;
    return undefined;
  }

  parseIndex(token: string): number | undefined {
    const match = POSITIONAL_INDEX_REGEX.exec(token);
    if (match === null) return undefined;
    return parseOptInt(match.groups?.position);
  }

  parseSlice(token: string): SliceSelector | undefined {
    const match = SLICE_SELECTOR_REGEX.exec(token);
    if (match === null) return undefined;
    const groups = match.groups ?? {};
    const slice: SliceSelector = {
      start: parseOptInt(groups.start),
      end: parseOptInt(groups.end),
      step: parseOptInt(groups.step),
    };
    if (slice.start !== undefined || slice.end !== undefined || slice.step !== undefined) {
      return slice;
    }
    return undefined;
  }
}
