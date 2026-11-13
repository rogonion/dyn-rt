// Publishable facade for the dyn-rt TypeScript modules (ADR 0003): one
// package, subpath exports per module so consumers only bundle what they
// use. Mirrors packages/rust/dyn-rt.

export * as shared from "@dyn-rt/shared";
