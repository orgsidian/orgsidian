import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { commands, type StarterVaultKind } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { errorMessage, VaultPicker } from "@/components/settings/VaultPicker";
import { localTodayIso } from "@/components/editor/schedule";

/**
 * Story 6.2 (FR-18): the first-launch onboarding surface. Renders three
 * primary Starter Vault choices (Personal GTD, Student, Freelancer) plus a
 * secondary "Use my own folder" link, per the epic's v0.1 stand-in for the
 * (v0.5 Beta, Story 11.1) Empty Starter.
 *
 * **Scope (locked 2026-09-05):** Story 6.1 shipped only the Personal GTD and
 * Student generators — the Freelancer starter's ≥1-backlink AC depends on
 * Story 8.7's BacklinksPanel, which does not exist yet (see
 * `docs/user-guide/starter-vaults.md#deferred`). The Freelancer card renders
 * per the AC but is disabled with a "Coming soon" affordance rather than
 * wired to a generator that doesn't exist.
 *
 * Selecting Personal GTD or Student prompts for a target folder via
 * `tauri-plugin-dialog`, then calls `commands.generateStarterVault` (Story
 * 6.1's generator + a `designateVault` in one backend round trip). "Use my
 * own folder" reveals the existing Story 3.6 `VaultPicker` inline — the same
 * folder-choose + `designateVault` + scan-progress flow the Settings surface
 * uses, rather than a duplicate implementation.
 */
interface StarterVaultPickerProps {
  /** Called once a Vault is configured by any path (a starter or an existing
   *  folder) — the `/today` route uses this to dismiss the picker. */
  onVaultConfigured: () => void;
}

interface StarterOption {
  kind: StarterVaultKind;
  title: string;
  description: string;
}

const PRIMARY_OPTIONS: StarterOption[] = [
  {
    kind: "personalGtd",
    title: "Personal GTD",
    description:
      "Inbox, one active project with Next Actions, a journal, and a Someday/Maybe list — David Allen's Getting Things Done method.",
  },
  {
    kind: "student",
    title: "Student",
    description:
      "Inbox, one active course with assignments and readings, a journal, and a Someday list — shaped around a term's coursework rhythm.",
  },
];

export function StarterVaultPicker({ onVaultConfigured }: StarterVaultPickerProps) {
  // Which primary card's folder-choose + generate + designate flow is
  // in-flight — disables the other cards so a user can't fire two
  // designations at once (mirrors `VaultPicker`'s single in-flight scan).
  const [activeKind, setActiveKind] = useState<StarterVaultKind | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Reveals the embedded `VaultPicker` for the "Use my own folder" flow.
  const [ownFolder, setOwnFolder] = useState(false);

  async function chooseStarter(kind: StarterVaultKind) {
    setError(null);

    let selection: string | string[] | null;
    try {
      selection = await open({ directory: true, multiple: false });
    } catch (err) {
      // A dialog-plugin failure must surface via the same alert path, not a
      // swallowed unhandled rejection.
      setError(errorMessage(err));
      return;
    }
    if (typeof selection !== "string") {
      // Dialog dismissed — leave the picker up, nothing chosen.
      return;
    }

    setActiveKind(kind);
    try {
      await commands.generateStarterVault(kind, selection, localTodayIso());
      onVaultConfigured();
    } catch (err) {
      setActiveKind(null);
      setError(errorMessage(err));
    }
  }

  const busy = activeKind !== null;

  return (
    <section
      aria-labelledby="starter-vault-picker-heading"
      className="mx-auto max-w-2xl px-6 py-10"
    >
      <h1
        id="starter-vault-picker-heading"
        className="text-2xl font-semibold text-[var(--org-fg-default)]"
      >
        Welcome to Orgsidian
      </h1>
      <p className="mt-2 text-sm text-[var(--org-fg-muted)]">
        Choose a Starter Vault to see the workflow, not the syntax, in your first five minutes.
      </p>

      <div className="mt-6 grid gap-3 sm:grid-cols-3">
        {PRIMARY_OPTIONS.map((option) => (
          <button
            key={option.kind}
            type="button"
            disabled={busy}
            onClick={() => chooseStarter(option.kind)}
            className="cursor-pointer rounded-lg border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] p-4 text-left transition-colors outline-none hover:bg-[var(--org-bg-elevated)] focus-visible:ring-2 focus-visible:ring-[var(--org-border-focus)] disabled:cursor-not-allowed disabled:opacity-50"
          >
            <span className="block font-medium text-[var(--org-fg-default)]">
              {option.title}
              {activeKind === option.kind && (
                <span
                  aria-live="polite"
                  aria-busy="true"
                  className="ml-2 text-xs font-normal text-[var(--org-fg-subtle)]"
                >
                  Setting up…
                </span>
              )}
            </span>
            <span className="mt-1 block text-xs text-[var(--org-fg-muted)]">
              {option.description}
            </span>
          </button>
        ))}

        {/* Freelancer — locked scope decision (2026-09-05): the generator is
            deferred to post-Story-8.7 (BacklinksPanel). Rendered per the AC,
            disabled, never wired to a non-existent generator.

            A11y: NOT a native `disabled` button — that drops the element
            from the tab order entirely, so a keyboard/screen-reader user
            never discovers the card exists or why it's inert. Instead
            `aria-disabled="true"` keeps it focusable and announced as
            disabled, with no `onClick` wired (so activating it is already a
            no-op), and `aria-describedby` explicitly associates the visible
            "Coming soon" reason as the disabled-state description. */}
        <button
          type="button"
          aria-disabled="true"
          aria-describedby="freelancer-coming-soon-reason"
          className="cursor-not-allowed rounded-lg border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] p-4 text-left opacity-50"
        >
          <span className="block font-medium text-[var(--org-fg-default)]">
            Freelancer
            <span
              id="freelancer-coming-soon-reason"
              className="ml-2 text-xs font-normal text-[var(--org-fg-subtle)]"
            >
              Coming soon
            </span>
          </span>
          <span className="mt-1 block text-xs text-[var(--org-fg-muted)]">
            Project/client-centric Starter Vault with milestones, clocked time, and backlinks.
          </span>
        </button>
      </div>

      {error && (
        <p role="alert" className="mt-4 text-sm text-destructive">
          {error}
        </p>
      )}

      <div className="mt-6 border-t border-[var(--org-border-default)] pt-4">
        {ownFolder ? (
          <VaultPicker onDesignated={onVaultConfigured} />
        ) : (
          <Button
            type="button"
            variant="link"
            className="h-auto p-0 text-sm"
            disabled={busy}
            onClick={() => setOwnFolder(true)}
          >
            Use my own folder…
          </Button>
        )}
      </div>
    </section>
  );
}
