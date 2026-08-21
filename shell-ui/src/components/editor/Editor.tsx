import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { defaultKeymap } from "@codemirror/commands";
import { Compartment, type Text } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";

import { commands, type EditorMode } from "@/lib/tauri";

import { modeExtensions } from "./editorMode";
import { sourceFidelity } from "./decorations/sourceFidelity";
import { createSplitEditor, type SplitSurface } from "./SplitEditor";

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
 * Extensions common to every surface — the editable flag, the base keymap and
 * the editor face — shared verbatim by the single-view host and BOTH Split
 * panes so all three render with identical base behavior. Mode-specific
 * extensions are layered on top (via the `Compartment` for single views, via
 * `modeExtensions` per pane inside {@link createSplitEditor}).
 */
const baseEditorExtensions = [
  EditorView.editable.of(true),
  keymap.of(defaultKeymap),
  editorFontTheme,
  // Mode-independent: find/replace and clipboard operate on the source document
  // in every mode and in both Split panes (Story 4.3g / FR-3, FR-4).
  sourceFidelity(),
];

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
 * Modes (Stories 4.2 + 4.4, FR-3 + LD-6 + LD-40).
 *
 * The surface is created inside a `useEffect` and destroyed in that effect's
 * cleanup, so React 19 StrictMode's dev double-mount leaves exactly zero leaked
 * views. A `disposed` guard blocks surface creation and post-unmount state
 * updates when an async load resolves after the component has unmounted.
 *
 * Raw and Pseudo-WYSIWYG are a single `EditorView` whose mode-dependent
 * extensions live behind a `Compartment`, so switching between them reconfigures
 * the view in place (no buffer reload, so source-position and round-trip
 * fidelity are untouched). Raw renders org syntax-highlight tokens only; the
 * Pseudo-WYSIWYG decoration layer is excluded (see `editorMode.ts`).
 *
 * Split mode (Story 4.4) is a different DOM surface — a 50/50 two-view,
 * shared-buffer layout built by {@link createSplitEditor} (Raw left,
 * Pseudo-WYSIWYG right, one logical buffer). Crossing the Split boundary rebuilds
 * the surface, handing the live document across so the switch neither reloads
 * from disk nor drops unsaved edits.
 *
 * Source and mode reach the frontend only through the typed `commands.*` client
 * (never raw `invoke`, never `plugin-fs`/`plugin-store` directly).
 */
export function Editor({ filePath, ref }: EditorProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  // The live Split surface (two synced views) when the mode is "split", else
  // null. `viewRef` always points at the primary (left/Raw) view so the handle
  // and single-view teardown treat both surfaces uniformly.
  const splitRef = useRef<SplitSurface | null>(null);
  // One Compartment instance for the component's lifetime holds the
  // mode-dependent extensions so `setMode` can reconfigure the single-view host
  // in place. Lazily initialized so a re-render allocates no throwaway.
  const modeCompartmentRef = useRef<Compartment | null>(null);
  // Authoritative current mode (the handle getter reads this); `modeState`
  // mirrors it only to reflect `data-editor-mode` in the DOM.
  const modeRef = useRef<EditorMode>(DEFAULT_MODE);
  const [modeState, setModeState] = useState<EditorMode>(DEFAULT_MODE);
  const [error, setError] = useState<string | null>(null);

  // Destroy whichever surface is live (a Split surface tears down both of its
  // views + wrapper; otherwise the single view). Idempotent, so a StrictMode
  // remount starts clean and a double cleanup is harmless.
  const teardownSurface = useCallback(() => {
    if (splitRef.current !== null) {
      splitRef.current.destroy();
      splitRef.current = null;
    } else {
      viewRef.current?.destroy();
    }
    viewRef.current = null;
  }, []);

  // Build the surface for `mode` into `parent` from `doc`. Split builds the
  // two-view shared-buffer surface; every other mode builds the single view
  // with the mode extensions behind the Compartment (so raw <-> pseudoWysiwyg
  // can later reconfigure in place). `viewRef` always ends up on the primary
  // view so the handle and teardown are surface-agnostic.
  const buildSurface = useCallback(
    (mode: EditorMode, doc: string | Text, parent: HTMLElement) => {
      if (mode === "split") {
        const surface = createSplitEditor({
          parent,
          doc,
          baseExtensions: baseEditorExtensions,
        });
        splitRef.current = surface;
        viewRef.current = surface.primaryView;
        return;
      }
      const compartment = (modeCompartmentRef.current ??= new Compartment());
      splitRef.current = null;
      viewRef.current = new EditorView({
        parent,
        doc,
        extensions: [baseEditorExtensions, compartment.of(modeExtensions(mode))],
      });
    },
    [],
  );

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
        const parent = containerRef.current;
        const isSplitNow = splitRef.current !== null;
        const view = viewRef.current;
        if (!isSplitNow && mode !== "split" && view !== null) {
          // raw <-> pseudoWysiwyg: reconfigure the single view in place — no
          // rebuild, so source-position and round-trip fidelity are untouched.
          const compartment = (modeCompartmentRef.current ??= new Compartment());
          view.dispatch({
            effects: compartment.reconfigure(modeExtensions(mode)),
          });
        } else if (parent !== null && (view !== null || isSplitNow)) {
          // Crossing the Split boundary needs a different DOM surface (one view
          // <-> two). Hand the LIVE document over to the new surface so nothing
          // reloads from disk and no unsaved edit is lost.
          const doc: string | Text = view?.state.doc ?? "";
          teardownSurface();
          buildSurface(mode, doc, parent);
        }
        // Persist per-file via the typed client; a failure (e.g. no active
        // Vault) must not break the in-session switch.
        void commands.setEditorMode(mode, filePath).catch(() => {});
      },
    }),
    [filePath, buildSurface, teardownSurface],
  );

  useEffect(() => {
    // Guards the async gap: if the component unmounts (or the effect re-runs)
    // before the loads resolve, cleanup flips `disposed` so the resolved
    // callbacks create no surface and touch no state.
    let disposed = false;
    setError(null);

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
      // Build the persisted surface directly — a file whose stored mode is
      // "split" opens straight into the two-view surface (no single-view flash,
      // no reload). Every other mode opens the single view. Find/replace and
      // clipboard source-fidelity (Story 4.3g) are wired into every view inside
      // buildSurface, so they apply in single and split surfaces alike.
      buildSurface(mode, source, parent);
    }

    void load();

    return () => {
      // Idempotent teardown: destroy exactly the surface this effect created and
      // drop the references so a StrictMode remount starts clean.
      disposed = true;
      teardownSurface();
    };
  }, [filePath, buildSurface, teardownSurface]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full overflow-auto bg-[var(--org-bg-canvas)] text-[var(--org-fg-default)]"
      data-editor-mode={modeState}
      data-error={error ?? undefined}
    />
  );
}
