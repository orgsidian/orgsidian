// Implements FR-5 — active-keymap preference (Story 4.7).
//
// Which chord set is active is a GLOBAL editor preference (native default vs
// opt-in Emacs), not a per-file one — unlike the Editor Mode (Raw/Pseudo/Split),
// which persists per file through the typed `commands.setEditorMode` client.
//
// Scope & lifetime (UX spec Principle 3, echoed in epic-4-context "Default
// landing state"): native keybindings are an ABSOLUTE cold-start default and
// "semantic state resets" — the keymap choice is session-scoped, NOT inherited
// across a restart. So this store is deliberately in-memory only: enabling Emacs
// mode takes effect immediately and app-wide for the session, and a cold start
// always lands back on native. Cross-restart recall belongs to the future
// "Reopen last session" Settings opt-in, not here — and it must not reach for
// `tauri-plugin-store` from the frontend (the Editor forbids that; per-file
// prefs go through the typed command client, and no global-preference command
// exists yet).
//
// The Settings toggle (`KeybindingsSettings`) writes via {@link setKeymapMode};
// the `Editor` host subscribes via {@link onKeymapModeChange} and reconfigures
// its CM6 keybindings Compartment in place, so toggling never reloads the buffer
// (the "enabling/disabling must not lose buffer state" AC of 4.7). The event
// surface mirrors `schedule.ts`'s emitter so the whole app shares one idiom.

import { DEFAULT_KEYMAP, type KeymapAction } from "./default";
import { EMACS_KEYMAP } from "./emacs";

/** The active chord set: native default (cold-start) or opt-in Emacs. */
export type KeymapMode = "default" | "emacs";

// Session source of truth. Cold start = native default (UX Principle 3).
let current: KeymapMode = "default";

/** The current active keymap mode. */
export function getKeymapMode(): KeymapMode {
  return current;
}

/**
 * Set the active keymap mode and notify every subscriber. A no-op (no emit)
 * when the value is unchanged, so subscribers never churn on a redundant set.
 */
export function setKeymapMode(mode: KeymapMode): void {
  if (current === mode) return;
  current = mode;
  emit(mode);
}

/** Convenience: is Emacs mode currently active? */
export function isEmacsMode(): boolean {
  return current === "emacs";
}

/** The chord set for a mode — the single place that maps mode → actions. */
export function activeKeymap(mode: KeymapMode): readonly KeymapAction[] {
  return mode === "emacs" ? EMACS_KEYMAP : DEFAULT_KEYMAP;
}

// --- Change subscription (mirrors schedule.ts's listener surface) -----------

type KeymapModeListener = (mode: KeymapMode) => void;
const listeners = new Set<KeymapModeListener>();

/** Subscribe to keymap-mode changes; returns an idempotent unsubscribe. */
export function onKeymapModeChange(listener: KeymapModeListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function emit(mode: KeymapMode): void {
  for (const listener of [...listeners]) {
    try {
      listener(mode);
    } catch (error) {
      // eslint-disable-next-line no-console
      console.error("onKeymapModeChange listener threw", error);
    }
  }
}

/**
 * Test-only: reset to the cold-start default and drop every subscriber, so each
 * test starts from a clean slate (mirrors the real cold-start reset).
 */
export function __resetKeymapModeForTests(): void {
  current = "default";
  listeners.clear();
}
