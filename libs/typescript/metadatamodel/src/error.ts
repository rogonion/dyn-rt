// Closed error vocabulary for the metadatamodel module (ADR 0002), port of
// libs/rust/metadatamodel/src/error.rs. TS uses exceptions, so this is one
// Error subclass with a closed `kind` union.

export type MetadatamodelErrorKind =
  "InvalidModel" | "MissingProperty" | "CardinalityViolation" | "PathTemplateInvalid" | "GetColumnFields" | "FieldValue" | "Manipulation" | "Filter" | "Flatten" | "Unflatten";

export class MetadatamodelError extends Error {
  readonly kind: MetadatamodelErrorKind;

  private constructor(kind: MetadatamodelErrorKind, message: string) {
    super(message);
    this.name = "MetadatamodelError";
    this.kind = kind;
  }

  static invalidModel(message: string): MetadatamodelError {
    return new MetadatamodelError("InvalidModel", `invalid model: ${message}`);
  }

  static missingProperty(key: string): MetadatamodelError {
    return new MetadatamodelError("MissingProperty", `missing property: ${key}`);
  }

  static cardinalityViolation(message: string): MetadatamodelError {
    return new MetadatamodelError("CardinalityViolation", `cardinality violation: ${message}`);
  }

  static pathTemplateInvalid(message: string): MetadatamodelError {
    return new MetadatamodelError("PathTemplateInvalid", `path template invalid: ${message}`);
  }

  static getColumnFields(message: string): MetadatamodelError {
    return new MetadatamodelError("GetColumnFields", `get column fields: ${message}`);
  }

  static fieldValue(message: string): MetadatamodelError {
    return new MetadatamodelError("FieldValue", `field value: ${message}`);
  }

  static manipulation(message: string): MetadatamodelError {
    return new MetadatamodelError("Manipulation", `manipulation: ${message}`);
  }

  static filter(message: string): MetadatamodelError {
    return new MetadatamodelError("Filter", `filter: ${message}`);
  }

  static flatten(message: string): MetadatamodelError {
    return new MetadatamodelError("Flatten", `flatten: ${message}`);
  }

  static unflatten(message: string): MetadatamodelError {
    return new MetadatamodelError("Unflatten", `unflatten: ${message}`);
  }
}
