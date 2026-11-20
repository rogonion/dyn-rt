// Closed error vocabulary for the manipulation module (ADR 0002), port of
// libs/rust/manipulation/src/error.rs. TS uses exceptions instead of Rust's
// Result, so this is a single Error subclass with a closed `kind` union.

export type ManipulationErrorKind = "PathSegmentInvalid" | "ValueAtPathSegmentInvalid" | "ConversionFailed";

export class ManipulationError extends Error {
  readonly kind: ManipulationErrorKind;

  private constructor(kind: ManipulationErrorKind, message: string) {
    super(message);
    this.name = "ManipulationError";
    this.kind = kind;
  }

  /** The path (or one of its segments) is structurally invalid — e.g. an
   * unsupported segment for the value's kind, a slice bound out of range. */
  static pathSegmentInvalid(message: string): ManipulationError {
    return new ManipulationError("PathSegmentInvalid", `path segment invalid: ${message}`);
  }

  /** The value at a path segment is missing or not usable — e.g. a map key
   * that does not exist, an index out of bounds, a nil source. */
  static valueAtPathSegmentInvalid(message: string): ManipulationError {
    return new ManipulationError("ValueAtPathSegmentInvalid", `value at path segment invalid: ${message}`);
  }

  /** Converting the value to set (or a map key) into the destination shape
   * failed (wraps schema's conversion failure). */
  static conversionFailed(message: string): ManipulationError {
    return new ManipulationError("ConversionFailed", `conversion failed: ${message}`);
  }
}
