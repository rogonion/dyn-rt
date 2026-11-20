// Flat config (ESLint 10) for the whole TS workspace — one config at the
// root covers libs/typescript/* and packages/typescript/*.
// Type-checked rules are off on purpose: per-package `tsc --noEmit` (the
// `lint` script, run via `pnpm -r lint`) already covers type errors; eslint
// here covers style/correctness rules (ADR 0005: explicit over clever).

import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["**/dist/**", "**/node_modules/**", "**/coverage/**", "**/target/**"],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      // explicit, boring code (ADR 0005)
      eqeqeq: "error",
      "no-var": "error",
      "prefer-const": "error",
      semi: ["error", "always"],
      // tests/examples print via the harness; allow console
      "no-console": "off",
      // allow `any` where a boundary type genuinely needs it (rare — the
      // codebase prefers `unknown`); unused vars must still be prefixed _
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" }],
    },
  },
);
