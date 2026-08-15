/// <reference types="vitest" />
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

/**
 * Vitest config — Story 1.17 (LD-58 a11y gate scaffold).
 *
 * Scope: unit/integration tests for shell-ui internals (themes, utilities, hooks).
 * E2E + axe-core scenarios run via Playwright (see playwright.config.ts).
 *
 * Implements NFR-9 / LD-58.
 */
const rootDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  // Mirror the `@` → `src` alias from vite.config.ts so component tests can
  // resolve the same `@/…` imports the app uses (Story 3.6: first component
  // unit test that imports through the alias).
  resolve: {
    alias: { '@': path.resolve(rootDir, './src') },
  },
  test: {
    environment: 'jsdom',
    globals: false,
    include: ['src/**/*.test.{ts,tsx}'],
    exclude: ['e2e/**', 'node_modules/**', 'dist/**'],
  },
});
