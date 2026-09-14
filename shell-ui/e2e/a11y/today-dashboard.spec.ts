/**
 * @a11y — Today Dashboard keyboard-only happy-path scenario (LD-58 gate #2 + #3).
 *
 * STATUS: ACTIVE — Story 7.1 landed the `/today` Today Dashboard surface, so
 * this scaffold is now a real gate (was `test.fixme` under Story 1.17).
 *
 * Story 1.17 wired the LD-58 hard CI gate as six keyboard-only scaffolds; the
 * surfaces ship in downstream epics. This one exercises the real surface:
 *   - Gate #3: a keyboard-ONLY interaction (Tab to a section's collapsible
 *     chevron trigger, toggle it with the keyboard, confirm the persisted
 *     side-effect and that focus stays put — never `mouse.click`).
 *   - Gate #2: an axe-core WCAG 2.1 AA scan of the Today Dashboard surface.
 *
 * Playwright drives a plain browser with no Tauri host, so the Tauri IPC the
 * surface needs (`has_configured_vault`, `today_dashboard`) is mocked at
 * `window.__TAURI_INTERNALS__` — this exercises the REAL `TodayDashboard`
 * component + Radix `Collapsible` DOM, only the data boundary is stubbed.
 *
 * Implements NFR-9 / LD-58.
 */
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// A populated dashboard so every section renders a chevron header AND rows
// (an all-empty dashboard still renders the five triggers, but rows exercise
// more of the surface for the axe scan). Field order/shape matches
// `TodayDashboardDto` in `src/lib/tauri.ts`.
const DASHBOARD = {
  scheduled: [
    {
      headlineId: 1,
      filePath: 'work.org',
      title: 'Ship v0.1 Alpha',
      byteStart: 0,
      todoKeyword: 'TODO',
      scheduledDate: '2026-09-05',
      scheduledTime: null,
      deadlineDate: null,
      deadlineTime: null,
      overdue: false,
      agendaDate: '2026-09-05',
    },
  ],
  deadlines: [
    {
      headlineId: 2,
      filePath: 'work.org',
      title: 'File the report',
      byteStart: 40,
      todoKeyword: 'TODO',
      scheduledDate: null,
      scheduledTime: null,
      deadlineDate: '2026-09-01',
      deadlineTime: null,
      overdue: true,
      agendaDate: '2026-09-05',
    },
  ],
  todayTag: [
    {
      headlineId: 3,
      filePath: 'notes.org',
      title: 'Tagged for today',
      byteStart: 8,
      todoKeyword: null,
      scheduledDate: null,
      scheduledTime: null,
      deadlineDate: null,
      deadlineTime: null,
      overdue: false,
      agendaDate: '2026-09-05',
    },
  ],
  inbox: [
    {
      headlineId: 4,
      filePath: 'inbox.org',
      title: 'Captured note',
      byteStart: 0,
      todoKeyword: null,
    },
  ],
  activeClock: {
    headlineId: 5,
    filePath: 'work.org',
    title: 'Deep work',
    byteStart: 12,
    startAt: '2026-09-05T09:00:00',
  },
};

test.describe('@a11y Today Dashboard', () => {
  test('keyboard-only happy-path + axe-core scan', async ({ page }) => {
    // Stub the Tauri IPC boundary so the real surface renders: a configured
    // vault (skip the onboarding gate) and a populated dashboard.
    await page.addInitScript((dashboard) => {
      let nextId = 1;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (window as any).__TAURI_INTERNALS__ = {
        invoke: (cmd: string) => {
          if (cmd === 'has_configured_vault') return Promise.resolve(true);
          if (cmd === 'today_dashboard') return Promise.resolve(dashboard);
          // Story 7.2 added the section-collapse prefs read on mount; the
          // surface stays on "Loading…" (and no section triggers render) until
          // it resolves to a real prefs object — a `null` resolve is NOT the
          // fall-back path. Return all-expanded so every section is open.
          if (cmd === 'get_today_dashboard_prefs')
            return Promise.resolve({
              scheduled: false,
              deadline: false,
              todayTag: false,
              inboxPreview: false,
              activeClock: false,
            });
          // Any other command the co-hosted placeholder panels might issue:
          // resolve benignly so nothing throws during the scan.
          return Promise.resolve(null);
        },
        // Minimal shim so the event API (if touched) does not explode.
        transformCallback: (cb: unknown, once = false) => {
          const id = nextId++;
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          (window as any)[`_${id}`] = (payload: unknown) => {
            if (once) delete (window as any)[`_${id}`];
            return typeof cb === 'function' ? (cb as (p: unknown) => unknown)(payload) : undefined;
          };
          return id;
        },
      };
    }, DASHBOARD);

    await page.goto('/today');

    // The REAL Today Dashboard surface has rendered (not the loading/onboarding
    // states, not an error alert).
    await expect(page.getByRole('heading', { level: 1, name: 'Today' })).toBeVisible();

    const scheduled = page.getByRole('button', { name: /Scheduled/ });
    await expect(scheduled).toBeVisible();
    // Default-expanded, so its row is on screen.
    await expect(scheduled).toHaveAttribute('aria-expanded', 'true');
    await expect(page.getByText('Ship v0.1 Alpha')).toBeVisible();

    // LD-58 gate #3 — keyboard-ONLY. Tab from the top until the first section
    // trigger holds focus (no mouse), then toggle it with the keyboard.
    for (let i = 0; i < 8; i++) {
      if (await scheduled.evaluate((el) => el === document.activeElement)) break;
      await page.keyboard.press('Tab');
    }
    await expect(scheduled).toBeFocused();

    // Activate via the keyboard: the section collapses, focus STAYS on the
    // trigger (focus is not lost to <body>), and the persisted side-effect —
    // the row leaving the DOM — is observable.
    await page.keyboard.press('Enter');
    await expect(scheduled).toHaveAttribute('aria-expanded', 'false');
    await expect(scheduled).toBeFocused();
    await expect(page.getByText('Ship v0.1 Alpha')).toHaveCount(0);

    // Re-activating re-expands it — keyboard toggle is symmetric.
    await page.keyboard.press('Enter');
    await expect(scheduled).toHaveAttribute('aria-expanded', 'true');
    await expect(page.getByText('Ship v0.1 Alpha')).toBeVisible();

    // LD-58 gate #2 — axe-core WCAG 2.1 AA scan of the Today Dashboard surface.
    // Scoped to the dashboard `<section>`: the co-hosted VaultPicker / Settings
    // placeholder panels on this route belong to other stories and are outside
    // Story 7.1's LD-58 obligation. serious + critical violations fail;
    // best-practice tier is excluded per LD-58 line 1369 (avoid noise that
    // erodes the gate).
    const results = await new AxeBuilder({ page })
      .include('[aria-labelledby="today-dashboard-heading"]')
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();
    const blocking = results.violations.filter(
      (v) => v.impact === 'serious' || v.impact === 'critical',
    );
    expect(blocking).toEqual([]);
  });
});
