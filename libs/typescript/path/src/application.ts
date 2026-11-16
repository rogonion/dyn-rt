// The parse use cases (port of libs/rust/path/src/application.rs).
//
// Orchestration only — all matching mechanics come from the PathTokenizer
// port, so the steps stay explicit and in order (ADR 0005):
//
// 1. split the path by the recursive descent operator `..`
// 2. split each part by dot notation `.`
// 3. split each part into bracket contents and plain text
// 4. extract one PathSegment per token

import type {
  AccessStrategy,
  PathSegment,
  PathTokenizer,
  RecursiveDescentSegment,
  RecursiveDescentSegments,
} from "./domain.js";

/** Parse a JSONPath string into its deconstructed AST. */
export function parsePath(tokenizer: PathTokenizer, path: string): RecursiveDescentSegments {
  const segments: RecursiveDescentSegments = [];

  // Step 1: split by recursive descent `..`
  for (const descentPart of tokenizer.splitRecursiveDescent(path)) {
    const group: RecursiveDescentSegment = [];

    // Step 2: split by dot notation `.`
    for (const dotPart of tokenizer.splitDotNotation(descentPart)) {
      // Step 3: split into bracket contents and plain text
      for (const token of tokenizer.splitBrackets(dotPart)) {
        // Step 4: extract the segment for this token
        group.push(extractSegment(tokenizer, token));
      }
    }

    segments.push(group);
  }

  return segments;
}

/** Extract the PathSegment for a single token (one item out of step 3).
 *
 * The checks run in a fixed, explicit order: root, wildcard, union, quoted
 * key, positional index, slice, plain key, unrecognized. */
export function extractSegment(tokenizer: PathTokenizer, token: string): PathSegment {
  // union members arrive with surrounding whitespace after the comma
  const trimmed = token.trim();

  // JSONPath root
  if (trimmed === "$") {
    return { accessStrategy: "Mixed", selector: { kind: "Root" } };
  }

  // JSONPath wildcard
  if (trimmed === "*") {
    return { accessStrategy: "Mixed", selector: { kind: "Wildcard" } };
  }

  // Union selector (more than one comma-separated member)
  const unionParts = tokenizer.splitUnion(trimmed);
  if (unionParts.length > 1) {
    return extractUnionSegment(tokenizer, unionParts);
  }

  // Quoted text key
  const quoted = tokenizer.extractQuoted(trimmed);
  if (quoted !== undefined) {
    return { accessStrategy: "Named", selector: { kind: "Key", key: quoted } };
  }

  // Positional index
  const index = tokenizer.parseIndex(trimmed);
  if (index !== undefined) {
    return { accessStrategy: "Positional", selector: { kind: "Index", index } };
  }

  // Slice selector
  const slice = tokenizer.parseSlice(trimmed);
  if (slice !== undefined) {
    return { accessStrategy: "Positional", selector: { kind: "Slice", slice } };
  }

  // Plain text key
  if (trimmed !== "") {
    return { accessStrategy: "Named", selector: { kind: "Key", key: trimmed } };
  }

  // Nothing matched
  return { accessStrategy: "Mixed", selector: { kind: "Unrecognized" } };
}

/** Extract a union segment from its comma-separated member tokens.
 *
 * The union's access strategy is Mixed when any member is Mixed or when
 * members mix Positional and Named; otherwise it inherits the single member
 * kind present. */
function extractUnionSegment(
  tokenizer: PathTokenizer,
  unionParts: string[],
): PathSegment {
  const members: PathSegment[] = [];
  let hasMixed = false;
  let hasPositional = false;
  let hasNamed = false;

  for (const part of unionParts) {
    const member = extractSegment(tokenizer, part);
    if (member.accessStrategy === "Mixed") hasMixed = true;
    if (member.accessStrategy === "Positional") hasPositional = true;
    if (member.accessStrategy === "Named") hasNamed = true;
    members.push(member);
  }

  let accessStrategy: AccessStrategy;
  if (hasMixed || (hasPositional && hasNamed)) {
    accessStrategy = "Mixed";
  } else if (hasPositional) {
    accessStrategy = "Positional";
  } else if (hasNamed) {
    accessStrategy = "Named";
  } else {
    accessStrategy = "Mixed";
  }

  return { accessStrategy, selector: { kind: "Union", members } };
}
