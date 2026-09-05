// Implements FR-22 -- theme preference + instant switch (Story 6.7).
//
// The theme PREFERENCE (`dark` / `light` / `system`) is the Settings ->
// Appearance choice; the RESOLVED theme (always a concrete `dark` or `light`)
// is what actually lands on the DOM as `document.body.dataset.theme`, per the
// AC's literal mechanism. "system" is never written to the DOM -- it is
// resolved via `matchMedia('(prefers-color-scheme: dark)')` first, so
// `tokens.css` / `light.css` / `dark.css` only ever need to handle two
// concrete selectors.
//
// Scope & lifetime: mirrors `components/editor/keybindings/keymapMode.ts` --
// an in-memory, SESSION-ONLY source of truth, deliberately not persisted
// across a restart. Same reasoning as `keymapMode.ts`: no
// `tauri-plugin-store` reach from the frontend for a global preference (no
// command exists yet; per-file prefs go through the typed command client),
// and cold start always lands back on "system" (the OS choice) rather than a
// remembered explicit pick. Cross-restart recall belongs to a future
// Settings-persistence story, not here. Cold start is applied at module-import
// time (before React renders any content), so the resolved theme is in place
// before the first painted UI. NOTE: this import is a deferred ES module, so a
// dark-OS "system" user can briefly see the (empty) body in the `:root` light
// default before this runs; eliminating that last background-only flash would
// need an inline blocking <script> at the top of `index.html`'s <body> -- a
// scope call left to review since there is no content to flash yet.
//
// The event surface mirrors `keymapMode.ts`'s listener idiom so the whole app
// shares one idiom for "global preference + live subscribers".

/** The user's Settings -> Appearance choice. */
export type ThemePreference = "dark" | "light" | "system";

/** The concrete theme actually applied to the DOM (never "system"). */
export type ResolvedTheme = "dark" | "light";

// Session source of truth. Cold start = system preference.
let preference: ThemePreference = "system";

const prefersDarkQuery: MediaQueryList | null =
  typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

function systemTheme(): ResolvedTheme {
  return prefersDarkQuery?.matches ? "dark" : "light";
}

function resolve(pref: ThemePreference): ResolvedTheme {
  return pref === "system" ? systemTheme() : pref;
}

function applyToDom(resolved: ResolvedTheme): void {
  // Guard `document.body` too, not just `document`: the cold-start apply runs
  // at module-import time, so a non-deferred or head-injected import context
  // (or a non-browser render) could reach here before <body> is parsed.
  if (typeof document !== "undefined" && document.body) {
    document.body.dataset.theme = resolved;
  }
}

/** The current Settings -> Appearance preference. */
export function getThemePreference(): ThemePreference {
  return preference;
}

/** The concrete theme currently applied ("system" already resolved). */
export function getResolvedTheme(): ResolvedTheme {
  return resolve(preference);
}

/**
 * Set the Settings -> Appearance preference. Applies instantly
 * (`document.body.dataset.theme = "dark" | "light"`) and notifies every
 * subscriber. A no-op (no DOM write, no emit) when the preference is
 * unchanged, so subscribers never churn on a redundant set.
 */
export function setThemePreference(pref: ThemePreference): void {
  if (preference === pref) return;
  preference = pref;
  applyToDom(resolve(pref));
  emit();
}

// Apply the cold-start default immediately so the first paint already
// carries the resolved theme.
applyToDom(resolve(preference));

// A "system" preference tracks live OS changes -- an explicit dark/light
// choice does not (the whole point of overriding the system preference).
prefersDarkQuery?.addEventListener("change", () => {
  if (preference !== "system") return;
  const next = systemTheme();
  // Same no-op contract as `setThemePreference`: a `change` event that leaves
  // the resolved theme unchanged writes nothing and does not churn subscribers.
  if (document.body?.dataset.theme === next) return;
  applyToDom(next);
  emit();
});

// --- Change subscription (mirrors keymapMode.ts's listener surface) --------

type ThemeChangeListener = (resolved: ResolvedTheme, preference: ThemePreference) => void;
const listeners = new Set<ThemeChangeListener>();

/** Subscribe to theme changes; returns an idempotent unsubscribe. */
export function onThemeChange(listener: ThemeChangeListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function emit(): void {
  const resolved = resolve(preference);
  for (const listener of [...listeners]) {
    try {
      listener(resolved, preference);
    } catch (error) {
      // eslint-disable-next-line no-console
      console.error("onThemeChange listener threw", error);
    }
  }
}

/**
 * Test-only: reset to the cold-start default and drop every subscriber, so
 * each test starts from a clean slate (mirrors the real cold-start reset).
 */
export function __resetThemeModeForTests(): void {
  preference = "system";
  listeners.clear();
  applyToDom(resolve(preference));
}
