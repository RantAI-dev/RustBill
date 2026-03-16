import nextConfig from "eslint-config-next";
import tseslint from "typescript-eslint";

const eslintConfig = [
  ...nextConfig,
  ...tseslint.configs.recommended,
  {
    rules: {
      // Allow `any` in type assertions and function parameters (common in API routes)
      "@typescript-eslint/no-explicit-any": "warn",
      // Allow unused vars prefixed with _
      "@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      // Allow setState in effects for pagination reset patterns
      "react-hooks/set-state-in-effect": "off",
      // Allow Date.now/Math.random in useMemo and component-level init
      "react-hooks/purity": "off",
    },
  },
];

export default eslintConfig;
