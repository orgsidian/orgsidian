/**
 * @a11y — Agenda keyboard-only happy-path scenario (LD-58 gate #2 + #3).
 *
 * STATUS: SCAFFOLD — passes via test.fixme until Story 6.3 (Today view) /
 * Story 6.4 (Week view) lands the surface.
 *
 * Story 1.17 wires the LD-58 hard CI gate; the 6 keyboard-only scenarios are
 * shipped as scaffolds (test.fixme) because the surfaces themselves ship in
 * downstream epics. When Story 6.3 / Story 6.4 land, replace test.fixme with
 * test, implement the keyboard-only path, and confirm AxeBuilder passes.
 *
 * Implements NFR-9 / LD-58 (Story 1.17 scaffold).
 */
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('@a11y Agenda', () => {
  test.fixme('keyboard-only happy-path + axe-core scan', async ({ page }) => {
    // TODO(Story 6.3 / Story 6.4): navigate to /agenda/today, perform a
    // representative action via page.keyboard only (NO mouse.click), and
    // assert the persisted side-effect.

    await page.goto('/agenda/today');

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
