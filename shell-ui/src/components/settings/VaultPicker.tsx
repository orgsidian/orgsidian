import { useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { commands } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { IndexScanProgress, type ScanPhase } from "@/components/settings/IndexScanProgress";

/**
 * Best-effort extraction of a human-readable message from a thrown command
 * error. `ErrorHandlingMode::Throw` throws the serialized `OrgError`
 * (`{ kind, reason }`); fall back to `String(err)` for anything else.
 */
function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "reason" in err) {
    return String((err as { reason: unknown }).reason);
  }
  return String(err);
}

/**
 * FR-15 Vault designation surface (architecture.md:1058): a folder picker that
 * calls `commands.designateVault`, with the non-modal scan-progress panel
 * beneath it. Local `useState` only — no state library (shell-ui has none).
 *
 * `designateVault` resolves only when the initial scan finishes, so the phase
 * transitions to `complete` (or `cancelled`, if the user hit Cancel) on
 * resolution; `IndexProgress` events stream the live counts in the meantime.
 */
export function VaultPicker() {
  const [phase, setPhase] = useState<ScanPhase>("idle");
  const [error, setError] = useState<string | null>(null);
  // A ref (not state) so the value read AFTER the await is the live one — a
  // Cancel click during the scan must not be lost to a stale closure.
  const cancelRequested = useRef(false);

  async function chooseVault() {
    setError(null);
    const selection = await open({ directory: true, multiple: false });
    if (typeof selection !== "string") {
      // Dialog dismissed (null) — leave the current phase untouched.
      return;
    }

    cancelRequested.current = false;
    setPhase("indexing");
    try {
      await commands.designateVault(selection);
      setPhase(cancelRequested.current ? "cancelled" : "complete");
    } catch (err) {
      setPhase("idle");
      setError(errorMessage(err));
    }
  }

  return (
    <section className="mt-6" aria-labelledby="vault-picker-heading">
      <h2 id="vault-picker-heading" className="text-lg font-medium">
        Vault
      </h2>
      <p className="mt-1 text-sm text-muted-foreground">
        Choose a folder of <code>.org</code> files to index.
      </p>

      <Button
        type="button"
        className="mt-3"
        onClick={chooseVault}
        disabled={phase === "indexing"}
      >
        Choose Vault folder…
      </Button>

      {error && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {error}
        </p>
      )}

      <IndexScanProgress
        phase={phase}
        onCancelRequested={() => {
          cancelRequested.current = true;
        }}
      />
    </section>
  );
}
