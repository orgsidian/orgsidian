import { useEffect, useImperativeHandle, useRef, useState } from "react";
import { defaultKeymap } from "@codemirror/commands";
import { Compartment } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";

import { commands, type EditorMode } from "@/lib/tauri";

import { modeExtensions } from "./editorMode";

/**
 * Cold-start default Editor Mode (LD, UX default landing state): Pseudo-WYSIWYG.
 * Used when a file has no persisted choice, or when the mode cannot be read
 * (e.g. no active Vault) — never a hard failure.
 */
const DEFAULT_MODE: EditorMode = "pseudoWysiwyg";

/**
 * IBM Plex Mono editor face (LD-6 styling contract) with a monospace fallback
 * stack; the embedded font file lands in a later themes story.
 */
const editorFontTheme = EditorView.theme({
  "&": {
    fontFamily:
      '"IBM Plex Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
  },
});

/**
 * Imperative handle exposed on the `ref` prop. Story 4.2 adds the live Editor
 * Mode plus a `setMode` action; consumers reach the CM6 buffer through `view`.
 * CM6 remains the single owner of the open file's state.
 */
export interface EditorHandle {
  /** The live CM6 view, or `null` before creation / after `destroy()`. */
  view: EditorView | null;
  /** Move keyboard focus into the editor (no-op when no view exists). */
  focus: () => void;
  /** The current Editor Mode (Raw / Pseudo-WYSIWYG / Split). */
  mode: EditorMode;
  /**
   * Switch the Editor Mode: reconfigure the live view (no buffer reload) and
   * persist the choice per-file via `commands.setEditorMode` (LD-40).
   */
  setMode: (mode: EditorMode) => void;
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
 * StrictMode-safe CodeMirror 6 host (Story 4.1) with org-mode-aware Editor
 * Modes (Story 4.2, FR-3 + LD-6 + LD-40).
 *
 * The `EditorView` is created inside a `useEffect` and destroyed in that
 * effect's cleanup, so React 19 StrictMode's dev double-mount leaves exactly
 * zero leaked views. A `disposed` guard blocks view creation and post-unmount
 * state updates when an async load resolves after the component has unmounted.
 *
 * Mode-dependent extensions live behind a `Compartment`, so switching modes
 * reconfigures the view in place (no buffer reload, so source-position and
 * round-trip fidelity are untouched). Raw mode renders org syntax-highlight
 * tokens only — the Pseudo-WYSIWYG decoration layer is excluded (see
 * `editorMode.ts`).
 *
 * Source and mode reach the frontend only through the typed `commands.*` client
 * (never raw `invoke`, never `plugin-fs`/`plugin-store` directly).
 */
export function Editor({ filePath, ref }: EditorProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  // One Compartment instance for the component's lifetime holds the
  // mode-dependent extensions so `setMode` can reconfigure them in place.
  // Lazily initialized so a re-render does not allocate a throwaway Compartment.
  const modeCompartmentRef = useRef<Compartment | null>(null);
  const modeCompartment: Compartment = (modeCompartmentRef.current ??=
    new Compartment());
  // Authoritative current mode (the handle getter + compartment read this);
  // `modeState` mirrors it only to reflect `data-editor-mode` in the DOM.
  const modeRef = useRef<EditorMode>(DEFAULT_MODE);
  const [modeState, setModeState] = useState<EditorMode>(DEFAULT_MODE);
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
      get mode() {
        return modeRef.current;
      },
      setMode(mode: EditorMode) {
        modeRef.current = mode;
        setModeState(mode);
        const view = viewRef.current;
        if (view !== null) {
          view.dispatch({
            effects: modeCompartment.reconfigure(modeExtensions(mode)),
          });
        }
        // Persist per-file via the typed client; a failure (e.g. no active
        // Vault) must not break the in-session switch.
        void commands.setEditorMode(mode, filePath).catch(() => {});
      },
    }),
    [filePath],
  );

  useEffect(() => {
    // Guards the async gap: if the component unmounts (or the effect re-runs)
    // before the loads resolve, cleanup flips `disposed` so the resolved
    // callbacks create no view and touch no state.
    let disposed = false;
    setError(null);
    const compartment = modeCompartment;

    async function load() {
      // Resolve the persisted per-file mode first; absent choice / no Vault /
      // read failure all fall back to the cold-start default (never throws).
      let mode: EditorMode = DEFAULT_MODE;
      try {
        const persisted = await commands.getEditorMode(filePath);
        if (persisted !== null) {
          mode = persisted;
        }
      } catch {
        // Keep DEFAULT_MODE.
      }
      if (disposed) return;

      let source: string;
      try {
        source = await commands.openFile(filePath);
      } catch (err) {
        if (!disposed) {
          setError(err instanceof Error ? err.message : String(err));
        }
        return;
      }

      const parent = containerRef.current;
      if (disposed || parent === null) return;

      modeRef.current = mode;
      setModeState(mode);
      viewRef.current = new EditorView({
        parent,
        doc: source,
        extensions: [
          EditorView.editable.of(true),
          keymap.of(defaultKeymap),
          editorFontTheme,
          compartment.of(modeExtensions(mode)),
        ],
      });
    }

    void load();

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
      data-editor-mode={modeState}
      data-error={error ?? undefined}
    />
  );
}
