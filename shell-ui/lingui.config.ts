import type { LinguiConfig } from "@lingui/conf";

const config: LinguiConfig = {
  locales: ["en"],
  sourceLocale: "en",
  catalogs: [
    {
      path: "<rootDir>/src/locales/{locale}/messages",
      include: ["<rootDir>/src"],
      exclude: ["**/*.test.{ts,tsx}", "**/routeTree.gen.ts", "**/node_modules/**"],
    },
  ],
  // PO (Gettext) is the default formatter in Lingui v6 — no explicit `format`
  // needed unless options must be passed via `formatter()` factory.
  compileNamespace: "ts",
};

export default config;
