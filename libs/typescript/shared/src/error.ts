// Closed error vocabulary for the json context (port of
// libs/rust/shared/src/error.rs). Adapters translate their native failures
// into these variants; nothing else crosses the core boundary.

export type JsonErrorKind = "InvalidPath" | "PathNotFound" | "TypeMismatch" | "InvalidValue";

/**
 * Errors produced by the json core (path parsing/resolution, object ops,
 * schema validation). The closed-variant enum becomes a class with a `kind`
 * discriminant plus the variant's fields.
 */
export class JsonError extends Error {
  readonly kind: JsonErrorKind;
  readonly path: string | undefined;
  readonly expected: string | undefined;
  readonly found: string | undefined;

  private constructor(kind: JsonErrorKind, message: string, path?: string, expected?: string, found?: string) {
    super(message);
    this.name = "JsonError";
    this.kind = kind;
    this.path = path;
    this.expected = expected;
    this.found = found;
  }

  /** A JSONPath string could not be parsed. */
  static invalidPath(path: string): JsonError {
    return new JsonError("InvalidPath", `invalid JSON path: ${path}`, path);
  }

  /** A parsed path does not resolve against the document. */
  static pathNotFound(path: string): JsonError {
    return new JsonError("PathNotFound", `path not found: ${path}`, path);
  }

  /** A path resolved to a value of an unexpected type. */
  static typeMismatch(path: string, expected: string, found: string): JsonError {
    return new JsonError("TypeMismatch", `type mismatch at ${path}: expected ${expected}, found ${found}`, path, expected, found);
  }

  /**
   * The document or one of its values is malformed for the requested
   * operation (e.g. indexing into a non-array, setting a key on a scalar).
   */
  static invalidValue(message: string): JsonError {
    return new JsonError("InvalidValue", `invalid value: ${message}`);
  }
}
