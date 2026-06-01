/**
 * Playwright config — Story 1.17 (LD-58 a11y gates #2 + #3).
 *
 * Scope: keyboard-only scenarios + axe-core integration for the 6 primary
 * surfaces (Today Dashboard, Agenda, Editor, Quick Capture, Settings, Graph View).
 * Story 1.17 ships scaffolds (test.fixme with TODO refs to downstream epics);
 * downstream stories (4.*, 6.*, 7.*, 8.*) fill in the assertions as their
 * surfaces ship.
 *
 * Implements NFR-9 / LD-58.
 */
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: 'http://localhost:1420',
    headless: true,
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
  ],
  webServer: {
    command: 'pnpm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
