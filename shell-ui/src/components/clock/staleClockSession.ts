// Story 7.7 (FR-8 / UJ-1) post-review fix: scope the prior-session
// running-clock prompt to ONE evaluation per app launch.
//
// `<StaleClockPrompt/>` is mounted inside the `/today` route, which TanStack
// Router unmounts and remounts on every navigation. Without a cross-mount
// guard the launch check re-runs on each remount — popping the modal (with a
// destructive "Discard") on a LIVE clock the user started seconds ago in the
// current session. The AC is a "prior-session running-clock prompt on launch",
// so the check must run at most ONCE per app launch, never per route mount.
//
// This module-level flag survives route remounts (the JS context lives for the
// whole app session) and resets only on an actual app relaunch (a fresh JS
// context). A clock STARTED in the current session therefore can never be
// re-surfaced as "prior-session": once launch has been evaluated the prompt is
// never re-armed. Mirrors the `themeMode.ts` module-singleton + test-reset
// convention (`__resetThemeModeForTests`).

let evaluatedThisSession = false;

/**
 * Claim the single per-launch stale-clock evaluation. Returns `true` exactly
 * once per app session — for the FIRST caller — and `false` for every
 * subsequent call (later route remounts). The flag is set synchronously so a
 * remount that races the first check's in-flight promise still skips.
 */
export function claimStaleClockEvaluation(): boolean {
  if (evaluatedThisSession) {
    return false;
  }
  evaluatedThisSession = true;
  return true;
}

/**
 * Whether the per-launch evaluation has already been claimed this session.
 * (Read-only; does not claim.)
 */
export function hasEvaluatedStaleClockThisSession(): boolean {
  return evaluatedThisSession;
}

/**
 * Test-only: reset to the cold-start (unevaluated) state so each test starts
 * from a fresh app-launch slate (mirrors `__resetThemeModeForTests`).
 */
export function __resetStaleClockSessionForTests(): void {
  evaluatedThisSession = false;
}
