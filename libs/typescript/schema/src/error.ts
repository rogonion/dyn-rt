// Closed error vocabulary for the schema module (port of
// libs/rust/schema/src/error.rs).

export type SchemaErrorKind = "ValidationFailed" | "ConversionFailed" | "SchemaPathError";

export class SchemaError extends Error {
  readonly kind: SchemaErrorKind;
  readonly path: string;

  private constructor(kind: SchemaErrorKind, message: string, path: string) {
    super(message);
    this.name = "SchemaError";
    this.kind = kind;
    this.path = path;
  }

  static validationFailed(message: string, path = "$"): SchemaError {
    return new SchemaError("ValidationFailed", `validation failed at ${path}: ${message}`, path);
  }

  static conversionFailed(message: string, path = "$"): SchemaError {
    return new SchemaError("ConversionFailed", `conversion failed at ${path}: ${message}`, path);
  }

  static schemaPathError(message: string, path = "$"): SchemaError {
    return new SchemaError("SchemaPathError", `schema path error at ${path}: ${message}`, path);
  }
}
