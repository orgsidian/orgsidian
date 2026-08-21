import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { defaultKeymap } from "@codemirror/commands";
import { Compartment, Prec, type Text } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { toast } from "sonner";

import { commands, type EditorMode } from "@/lib/tauri";

import { ConflictBanner } from "./ConflictBanner";
import { modeExtensions } from "./editorMode";
import { sourceFidelity } from "./decorations/sourceFidelity";
import { createSplitEditor, type SplitSurface } from "./SplitEditor";
import { buildDefaultKeymap } from "./keybindings/default";
import {
  activeKeymap,
  getKeymapMode,
  onKeymapModeChange,
  type KeymapMode,
} from "./keybindings/keymapMode";
import {
  currentPlanningValue,
  onPlanningRequested,
  setPlanning,
} from "./schedule";
import {
  OrgDatePicker,
  type OrgDatePickerValue,
  type OrgPlanningKind,
} from "../org/OrgDatePicker";

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
 * Story 4.6/4.7 (FR-5): the ACTIVE editor keymap for `mode` — the native
 * default set or the opt-in Emacs set — as a CM6 keymap extension.
 *
 * The single source of truth in `keybindings/{default,emacs}.ts` drives the
 * editor-owned chords (cycle TODO, toggle checkbox, Schedule, Deadline) plus the
 * reserved chords whose features ship in a later epic. Schedule/Deadline (Story
 * 4.8) publish a picker-open request on the shared surface; the host decides
 * whether to open the picker (Raw mode suppresses it for plain typing).
 * Find/replace (`sourceFidelity`) and the mode switch (`ModeSwitcher`, a global
 * listener) are bound by their owners, so this keymap does not re-emit them.
 * Reserved chords surface a "coming soon" toast rather than a silent no-op or a
 * fake implementation.
 *
 * Wrapped in `Prec.high` so the active keymap wins over CM6's baseline
 * `defaultKeymap` on any conflict — the "active keymap takes precedence" AC of
 * Story 4.7 (the native and Emacs sets never coexist: the host swaps this
 * behind a Compartment, so only one set is ever wired at a time).
 */
function activeKeybindings(mode: KeymapMode) {
  return Prec.high(
    keymap.of(
      buildDefaultKeymap({
        actions: activeKeymap(mode),
        onReserved: (action) => toast(`${action.label} — coming soon`),
      }),
    ),
  );
}

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
  /**
   * Notified whenever the live Editor Mode changes — on the initial
   * persisted-mode load and on every `setMode`. Lets a parent mirror the mode
   * into UI state (e.g. to drive `ModeSwitcher`'s active segment) without
   * duplicating buffer ownership (Story 4.5). Optional and additive: the host
   * stays the single owner of mode + buffer.
   */
  onModeChange?: (mode: EditorMode) => void;
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
export function Editor({ filePath, ref, onModeChange }: EditorProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  // Latest `onModeChange` in a ref so emitting it never forces the mode-setting
  // paths (setMode / async load) to depend on the callback identity.
  const onModeChangeRef = useRef(onModeChange);
  useEffect(() => {
    onModeChangeRef.current = onModeChange;
  });
  const viewRef = useRef<EditorView | null>(null);
  // The live Split surface (two synced views) when the mode is "split", else
  // null. `viewRef` always points at the primary (left/Raw) view so the handle
  // and single-view teardown treat both surfaces uniformly.
  const splitRef = useRef<SplitSurface | null>(null);
  // One Compartment instance for the component's lifetime holds the
  // mode-dependent extensions so `setMode` can reconfigure the single-view host
  // in place. Lazily initialized so a re-render allocates no throwaway.
  const modeCompartmentRef = useRef<Compartment | null>(null);
  // Story 4.7 (FR-5): a second Compartment holds the ACTIVE keybinding set
  // (native default vs opt-in Emacs) so toggling Emacs mode reconfigures the
  // live view(s) in place — no buffer reload, no lost edits. One instance for
  // the component's lifetime, shared by the single view and both Split panes.
  const keybindingsCompartmentRef = useRef<Compartment | null>(null);
  // The active keymap mode read once up front (and kept current by the
  // subscription below) so a surface built during load starts on the right set.
  const keymapModeRef = useRef<KeymapMode>(getKeymapMode());
  // Authoritative current mode (the handle getter reads this); `modeState`
  // mirrors it only to reflect `data-editor-mode` in the DOM.
  const modeRef = useRef<EditorMode>(DEFAULT_MODE);
  const [modeState, setModeState] = useState<EditorMode>(DEFAULT_MODE);
  const [error, setError] = useState<string | null>(null);
  // Story 4.8 (FR-9): the open date-picker request, or null when closed. Holds
  // the target view + kind and the Headline's existing value (for edit pre-fill).
  const [picker, setPicker] = useState<{
    kind: OrgPlanningKind;
    view: EditorView;
    initial: OrgDatePickerValue | null;
  } | null>(null);

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
      const keybindings = (keybindingsCompartmentRef.current ??=
        new Compartment());
      // Seed the keybindings compartment from the current active keymap mode so
      // a view built while Emacs mode is already on starts on the Emacs set.
      const keybindingsExt = keybindings.of(
        activeKeybindings(keymapModeRef.current),
      );
      if (mode === "split") {
        const surface = createSplitEditor({
          parent,
          doc,
          baseExtensions: [baseEditorExtensions, keybindingsExt],
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
        extensions: [
          baseEditorExtensions,
          keybindingsExt,
          compartment.of(modeExtensions(mode)),
        ],
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
        onModeChangeRef.current?.(mode);
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

  // Story 4.8 (FR-9): open the date picker when a Schedule/Deadline keybinding
  // fires — EXCEPT in Raw mode, where the AC calls for plain typing of
  // `SCHEDULED:`/`DEADLINE:` lines with no picker. Subscribed once; the request
  // carries the concrete pane view so Split's two panes both work.
  useEffect(() => {
    return onPlanningRequested(({ kind, view }) => {
      if (modeRef.current === "raw") return;
      setPicker({ kind, view, initial: currentPlanningValue(view.state, kind) });
    });
  }, []);

  // Story 4.7 (FR-5): reconfigure the keybindings Compartment on every live view
  // when the global Emacs-mode preference changes. Reconfiguring swaps ONLY the
  // active keymap — the document, selection, undo history and every other
  // extension are untouched — so enabling/disabling Emacs mode never reloads the
  // buffer or drops unsaved edits (the buffer-state AC). Both Split panes are
  // reconfigured because they are two views over the shared buffer.
  const applyKeymapMode = useCallback((mode: KeymapMode) => {
    keymapModeRef.current = mode;
    const compartment = keybindingsCompartmentRef.current;
    if (compartment === null) return; // no surface yet; buildSurface will seed it
    const effect = compartment.reconfigure(activeKeybindings(mode));
    for (const view of [
      viewRef.current,
      splitRef.current?.secondaryView ?? null,
    ]) {
      if (view !== null) view.dispatch({ effects: effect });
    }
  }, []);

  useEffect(() => {
    // Re-sync in case the preference changed between the initial ref read and
    // this subscription, then reconfigure the live surface on every later change.
    applyKeymapMode(getKeymapMode());
    return onKeymapModeChange(applyKeymapMode);
  }, [applyKeymapMode]);

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
      onModeChangeRef.current?.(mode);
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
      className="relative h-full w-full"
      data-editor-mode={modeState}
      data-error={error ?? undefined}
    >
      {/* Story 5.5 (FR-16): the dirty-buffer external-conflict banner. Renders
          nothing until a `conflict-detected` event lands for this file; then it
          surfaces the blocked-save warning + the discard / view-file actions,
          inline at the top of the editor surface (never a modal — Epic 9). */}
      <ConflictBanner filePath={filePath} />
      <div
        ref={containerRef}
        className="h-full w-full overflow-auto bg-[var(--org-bg-canvas)] text-[var(--org-fg-default)]"
      />
      {picker !== null && (
        // Story 4.8 (FR-9): the inline picker overlays the editor. Enter commits
        // (writes via commands.setScheduled), Esc cancels; either way focus
        // returns to the editor.
        <div className="absolute left-1/2 top-4 z-10 -translate-x-1/2">
          <OrgDatePicker
            kind={picker.kind}
            initial={picker.initial}
            onConfirm={(value) => {
              const { view, kind } = picker;
              setPicker(null);
              void setPlanning(view, kind, value).finally(() => view.focus());
            }}
            onCancel={() => {
              const { view } = picker;
              setPicker(null);
              view.focus();
            }}
          />
        </div>
      )}
    </div>
  );
}
