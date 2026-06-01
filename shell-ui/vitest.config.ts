/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';

/**
 * Vitest config — Story 1.17 (LD-58 a11y gate scaffold).
 *
 * Scope: unit/integration tests for shell-ui internals (themes, utilities, hooks).
 * E2E + axe-core scenarios run via Playwright (see playwright.config.ts).
 *
 * Implements NFR-9 / LD-58.
 */
export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: false,
    include: ['src/**/*.test.{ts,tsx}'],
    exclude: ['e2e/**', 'node_modules/**', 'dist/**'],
  },
});
