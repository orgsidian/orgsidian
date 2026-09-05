// Implements FR-22 -- Settings -> Appearance theme toggle (Story 6.7).
//
// Dark / Light / System default, backed by `themes/themeMode.ts`. Local state
// mirrors the module-level store (seeded from `getThemePreference` /
// `getResolvedTheme`, kept in sync via `onThemeChange`) so the radios and the
// "Currently applied" line reflect a change made anywhere in the app (mirrors
// the `KeybindingsSettings` / active-keymap-store pattern).
//
// A11y: native `<input type="radio">` elements sharing one `name`, wrapped in
// a `<fieldset>` with a visually-hidden `<legend>` -- the group semantics come
// from the platform, no `role="radiogroup"` needed. "Currently applied" is
// `aria-live="polite"` text (never color alone) so a system-preference change
// while the panel is open is announced.

import { useEffect, useId, useState } from "react";

import {
  getResolvedTheme,
  getThemePreference,
  onThemeChange,
  setThemePreference,
  type ResolvedTheme,
  type ThemePreference,
} from "@/themes/themeMode";

interface ThemeOption {
  value: ThemePreference;
  label: string;
  description: string;
}

const OPTIONS: readonly ThemeOption[] = [
  { value: "light", label: "Light", description: "Always use the light theme." },
  { value: "dark", label: "Dark", description: "Always use the dark theme." },
  {
    value: "system",
    label: "System default",
    description: "Match the operating system's setting, and follow it live.",
  },
];

interface AppearanceSettingsProps {
  className?: string;
}

/**
 * Settings -> Appearance (Story 6.7, FR-22): dark / light / system-default
 * theme toggle. Selecting an option calls {@link setThemePreference}, which
 * applies `document.body.dataset.theme` instantly -- no reload, no
 * re-mount, just a CSS custom-property cascade re-evaluation.
 */
export function AppearanceSettings({ className }: AppearanceSettingsProps) {
  const [preference, setPreference] = useState<ThemePreference>(() => getThemePreference());
  const [resolved, setResolved] = useState<ResolvedTheme>(() => getResolvedTheme());

  // Reflect changes made elsewhere (or by a live system-preference change
  // while "System default" is selected) -- one store, one truth.
  useEffect(
    () =>
      onThemeChange((nextResolved, nextPreference) => {
        setResolved(nextResolved);
        setPreference(nextPreference);
      }),
    [],
  );

  const groupId = useId();
  const headingId = `${groupId}-heading`;

  return (
    <section className={className} aria-labelledby={headingId}>
      <h2 id={headingId} className="text-lg font-medium">
        Appearance
      </h2>
      <p className="mt-1 text-sm text-muted-foreground">
        Choose the color theme. Switching applies instantly.
      </p>

      <fieldset className="mt-4 m-0 space-y-3 border-0 p-0">
        {/* More specific than the section heading so a screen reader announces
            "Appearance" region → "Theme" group, not "Appearance" twice. */}
        <legend className="sr-only">Theme</legend>
        {OPTIONS.map((option) => {
          const inputId = `${groupId}-${option.value}`;
          return (
            <div key={option.value} className="flex items-start gap-3">
              <input
                type="radio"
                id={inputId}
                name={`${groupId}-theme`}
                data-testid={`theme-option-${option.value}`}
                checked={preference === option.value}
                onChange={() => setThemePreference(option.value)}
                className="mt-0.5"
              />
              <label htmlFor={inputId} className="text-sm">
                <span className="font-medium text-[var(--org-fg-default)]">{option.label}</span>
                <span className="block text-xs text-muted-foreground">{option.description}</span>
              </label>
            </div>
          );
        })}
      </fieldset>

      <p
        className="mt-4 text-xs text-muted-foreground"
        data-testid="resolved-theme"
        aria-live="polite"
      >
        Currently applied: {resolved === "dark" ? "Dark" : "Light"}
        {preference === "system" && " (from system)"}
      </p>
    </section>
  );
}

// i18n note: labels are plain strings, matching KeybindingsSettings / VaultPicker;
// UI-string extraction is a dedicated later pass.
