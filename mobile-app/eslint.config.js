const js = require("@eslint/js");
const jsdoc = require("eslint-plugin-jsdoc");
const reactHooks = require("eslint-plugin-react-hooks");
const tsdoc = require("eslint-plugin-tsdoc");
const globals = require("globals");
const tseslint = require("typescript-eslint");

module.exports = tseslint.config(
  { ignores: ["node_modules", "dist", "web-build", ".expo", "android", "ios", "coverage", "src/api/schema.d.ts", "expo-env.d.ts"] },
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
    files: ["__tests__/**", "jest.setup.ts"],
    languageOptions: { globals: { ...globals.jest, ...globals.node } },
    rules: { "jsdoc/require-jsdoc": "off" },
  },
  {
    files: ["*.config.js"],
    languageOptions: { sourceType: "commonjs", globals: { ...globals.node } },
    rules: { "@typescript-eslint/no-require-imports": "off" },
  },
);
