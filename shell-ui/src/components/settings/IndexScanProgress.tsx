import { useEffect, useState } from "react";

import { commands, events, type IndexProgress } from "@/lib/tauri";
import { Button } from "@/components/ui/button";

/**
 * Lifecycle of the initial scan, owned by the parent that drives
 * `commands.designateVault` (VaultPicker). The `IndexProgress` event carries
 * only counts, so "complete" vs "cancelled" is decided by the parent (did the
 * user click Cancel?), not by the payload.
 */
export type ScanPhase = "idle" | "indexing" | "complete" | "cancelled";

interface IndexScanProgressProps {
  /** Current scan lifecycle phase. */
  phase: ScanPhase;
  /** Notify the parent that cancellation was requested (so it can resolve the
   *  designation into the "cancelled" phase). */
  onCancelRequested: () => void;
}

/**
 * Non-modal initial-scan progress surface (LD-42): the enumerated
 * "N of M files indexed, X errors" panel plus a Cancel button, subscribed to
 * the `index-progress` event. Not a dialog — it renders inline. The live region
 * is `aria-live="polite"` and counts are text (never color alone) per LD-58.
 */
export function IndexScanProgress({ phase, onCancelRequested }: IndexScanProgressProps) {
  const [progress, setProgress] = useState<IndexProgress | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void events.indexProgress
      .listen((event) => setProgress(event.payload))
      .then((dispose) => {
        if (disposed) {
          dispose();
        } else {
          unlisten = dispose;
        }
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  if (phase === "idle") {
    return null;
  }

  const current = progress?.current ?? 0;
  const total = progress?.total ?? 0;
  const errors = progress?.errors ?? 0;

  function cancel() {
    onCancelRequested();
    void commands.cancelIndexScan();
  }

  return (
    <div className="mt-4 border-l-2 border-primary pl-3">
      {/* Live counts. aria-live=polite so a screen reader announces each
          checkpoint update without stealing focus. */}
      <p aria-live="polite" className="text-sm">
        {phase === "indexing" &&
          `Indexing… ${current} of ${total} files indexed, ${errors} errors`}
        {phase === "complete" &&
          `Indexed ${current} of ${total} files${errors > 0 ? `, ${errors} errors` : ""}`}
        {phase === "cancelled" &&
          `Cancelled — ${current} of ${total} indexed. Resume any time.`}
      </p>

      {errors > 0 && (
        <p className="text-sm text-muted-foreground">{errors} files unparseable</p>
      )}

      {phase === "indexing" && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="mt-2"
          onClick={cancel}
        >
          Cancel
        </Button>
      )}
    </div>
  );
}
