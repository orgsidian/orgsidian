/**
 * @a11y — Quick Capture keyboard-only happy-path scenario (LD-58 gate #2 + #3).
 *
 * STATUS: SCAFFOLD — passes via test.fixme until Story 8.1 lands the surface.
 *
 * Story 1.17 wires the LD-58 hard CI gate; the 6 keyboard-only scenarios are
 * shipped as scaffolds (test.fixme) because the surfaces themselves ship in
 * downstream epics. When Story 8.1 lands, replace test.fixme with test,
 * implement the keyboard-only path, and confirm AxeBuilder passes.
 *
 * NOTE: Quick Capture is a separate Tauri window (Story 8.1). The test
 * pattern (e.g., booting the Tauri shell vs. a web fallback route) is to be
 * decided when Story 8.1 lands — this scaffold uses a placeholder web route
 * (`/quick-capture`) which Story 8.1 will replace with the correct harness.
 *
 * Implements NFR-9 / LD-58 (Story 1.17 scaffold).
 */
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('@a11y Quick Capture', () => {
  test.fixme('keyboard-only happy-path + axe-core scan', async ({ page }) => {
    // TODO(Story 8.1): decide on the Tauri-window test pattern — may require
    // booting the Tauri shell rather than the Vite dev server. Navigate to
    // the Quick Capture surface, perform a representative action via
    // page.keyboard only (NO mouse.click), and assert the persisted
    // side-effect.
    //
    // NOTE: the route literal here is intentionally NOT pre-bound. Quick
    // Capture is a separate Tauri window and the AC4 mapping table left
    // the placeholder TBD. When un-fixme-ing this test, Story 8.1 must
    // choose either a Tauri-shell harness or a web-fallback route, then
    // add a surface-discriminator assertion (e.g.,
    //   await expect(page.getByRole('heading', { name: /quick capture/i })).toBeVisible();
    // ) BEFORE the axe scan so the gate cannot pass green against a 404
    // / placeholder.
    //
    // await page.goto('<route-or-tauri-window-pending — Story 8.1>');

    // LD-58 gate #3 — keyboard-only navigation.
    // Example: await page.keyboard.press('Tab'); await page.keyboard.press('Enter');

    // LD-58 gate #2 — axe-core WCAG 2.1 AA scan.
    // serious + critical violations fail; best-practice tier excluded
    // per LD-58 line 1369 (avoid noise that erodes the gate).
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();
    const blocking = results.violations.filter(
      (v) => v.impact === 'serious' || v.impact === 'critical',
    );
    expect(blocking).toEqual([]);
  });
});
