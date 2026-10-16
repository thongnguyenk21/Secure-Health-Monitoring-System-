import js from "@eslint/js";
import jsdoc from "eslint-plugin-jsdoc";
import reactHooks from "eslint-plugin-react-hooks";
import tsdoc from "eslint-plugin-tsdoc";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "node_modules", "src/api/schema.d.ts"] },
  js.configs.recommended,
  ...tseslint.configs.strict,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: { globals: { ...globals.browser } },
    plugins: { "react-hooks": reactHooks, tsdoc, jsdoc },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "tsdoc/syntax": "error",
      "@typescript-eslint/no-explicit-any": "error",
      "no-throw-literal": "error",
      // Every exported item carries a TSDoc summary (CODING_CONVENTIONS 2.3).
      "jsdoc/require-jsdoc": [
        "error",
        {
          publicOnly: true,
          require: { FunctionDeclaration: true, ArrowFunctionExpression: true, ClassDeclaration: true },
          contexts: ["TSTypeAliasDeclaration", "TSInterfaceDeclaration", "ExportNamedDeclaration > VariableDeclaration"],
        },
      ],
      // User-visible text lives in src/i18n/vi.ts, never inline in JSX.
      "no-restricted-syntax": [
        "error",
        {
          selector: "JSXText[value=/[A-Za-zÀ-ỹ]/]",
          message: "Put user-visible text in src/i18n/vi.ts.",
        },
      ],
    },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/test/**"],
    rules: { "jsdoc/require-jsdoc": "off" },
  },
  {
    files: ["*.config.{js,ts}"],
    languageOptions: { globals: { ...globals.node } },
  },
);
