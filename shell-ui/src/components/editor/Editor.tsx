import { useEffect, useImperativeHandle, useRef, useState } from "react";
import { defaultKeymap } from "@codemirror/commands";
import { EditorView, keymap } from "@codemirror/view";

import { commands } from "@/lib/tauri";

/**
 * Imperative handle exposed on the `ref` prop. Kept intentionally small for
 * Story 4.1: the live `EditorView` (or `null` before it exists / after
 * teardown) plus a `focus()` convenience. Consumers reach the CM6 buffer
 * through `view`; CM6 remains the single owner of the open file's state.
 */
export interface EditorHandle {
  /** The live CM6 view, or `null` before creation / after `destroy()`. */
  view: EditorView | null;
  /** Move keyboard focus into the editor (no-op when no view exists). */
  focus: () => void;
}

interface EditorProps {
  /** Absolute path to the `.org` file whose source CM6 renders. */
  filePath: string;
  /**
   * React 19 ref-as-prop — a regular prop, NO `forwardRef`. Receives the
   * {@link EditorHandle}.
   */
  ref?: React.Ref<EditorHandle>;
}

/**
 * StrictMode-safe CodeMirror 6 host (Story 4.1, LD-6 + LD-2).
 *
 * The `EditorView` is created inside a `useEffect` and destroyed in that
 * effect's cleanup, so React 19 StrictMode's dev double-mount (mount → cleanup
 * → remount) leaves exactly zero leaked views: every created view is
 * `destroy()`ed exactly once. A `disposed` guard blocks view creation and
 * post-unmount state updates when the async `openFile` load resolves after the
 * component has already unmounted.
 *
 * Source is loaded only through the typed `commands.openFile` client (never raw
 * `invoke`, never `plugin-fs`) and rendered byte-faithful. This story ships the
 * lifecycle-safe primitive only — decorations, modes, and keybindings land in
 * later stories, so the extension set here is deliberately minimal.
 */
export function Editor({ filePath, ref }: EditorProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useImperativeHandle(
    ref,
    () => ({
      get view() {
        return viewRef.current;
      },
      focus() {
        viewRef.current?.focus();
      },
    }),
    [],
  );

  useEffect(() => {
    // Guards the async gap: if the component unmounts (or the effect re-runs)
    // before `openFile` resolves, the cleanup flips `disposed` so the resolved
    // callback creates no view and touches no state.
    let disposed = false;
    setError(null);

    void commands
      .openFile(filePath)
      .then((source) => {
        const parent = containerRef.current;
        if (disposed || parent === null) {
          return;
        }
        // Minimal, source-faithful, editable view. No `basicSetup`, no
        // language/highlight — that is Story 4.2+.
        viewRef.current = new EditorView({
          parent,
          doc: source,
          extensions: [EditorView.editable.of(true), keymap.of(defaultKeymap)],
        });
      })
      .catch((err: unknown) => {
        if (disposed) {
          return;
        }
        setError(err instanceof Error ? err.message : String(err));
      });

    return () => {
      // Idempotent teardown: destroy exactly the view this effect created and
      // drop the reference so a StrictMode remount starts clean.
      disposed = true;
      viewRef.current?.destroy();
      viewRef.current = null;
    };
  }, [filePath]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full overflow-auto bg-[var(--org-bg-canvas)] text-[var(--org-fg-default)]"
      data-error={error ?? undefined}
    />
  );
}
