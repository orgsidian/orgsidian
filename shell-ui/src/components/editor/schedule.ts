// Implements FR-9 — Schedule/Deadline editing controller (Story 4.8).
//
// This module is the glue between the FR-9 date picker (`components/org/
// OrgDatePicker.tsx`) and the buffer: it resolves the *current Headline* from
// the CM6 selection, calls the typed `commands.setScheduled` client (NEVER a
// raw `invoke`) to compute the byte-faithful planning-line edit, and applies
// that edit as ONE CM6 transaction tagged `input.set-planning` — the LD-26
// shared editor surface, the same pattern the checkbox/TODO widgets use.
//
// Two invariants live here:
//  1. Offset units. The Rust command speaks UTF-8 *byte* offsets; CM6 document
//     positions are UTF-16 code units. Every offset crossing the boundary is
//     converted (`jsIndexToUtf8Byte` / `utf8ByteToJsIndex`) so a headline with
//     non-ASCII text before it still edits the right bytes (FR-2 round-trip).
//  2. Raw-mode fallback. The picker is opened via the `onPlanningRequested`
//     surface; the host suppresses it in Raw mode, where the AC calls for plain
//     typing of `SCHEDULED:`/`DEADLINE:` lines with no picker. This module does
//     not open any UI itself — it only publishes the request.
//
// Timestamp semantics (parsing/formatting, recurring cookies) are NOT
// re-implemented here — the Rust semantic layer owns them. The one bit of
// TS-side reading is a delimiter-anchored date/time *extract* for pre-filling
// the picker on edit, mirroring the display-only extraction Story 4.3d already
// does in `decorations/timestamps.ts` (not a re-parse of the grammar).

import { type EditorState } from "@codemirror/state";
import { type EditorView, type KeyBinding } from "@codemirror/view";

import { commands, type PlanningEdit } from "@/lib/tauri";

import { type OrgDatePickerValue, type OrgPlanningKind } from "../org/OrgDatePicker";

/** `Transaction.userEvent` tag on every planning write (LD-6 / LD-26). */
export const PLANNING_USER_EVENT = "input.set-planning";

// --- UTF-8 byte <-> UTF-16 (JS string) offset conversion ------------------
// The buffer text is byte-identical on both sides; only the indexing unit
// differs. Both endpoints land on char boundaries (the Rust offsets come from
// newline finds and timestamp spans), so decoding a byte prefix is always
// valid. A discrete user action, so the O(n) walk is not a hot path.

const UTF8_ENCODER = new TextEncoder();
const UTF8_DECODER = new TextDecoder();

/** UTF-16 (JS string) index → UTF-8 byte offset into `source`. */
export function jsIndexToUtf8Byte(source: string, jsIndex: number): number {
  return UTF8_ENCODER.encode(source.slice(0, jsIndex)).length;
}

/** UTF-8 byte offset → UTF-16 (JS string) index into `source`. */
export function utf8ByteToJsIndex(source: string, byteOffset: number): number {
  const bytes = UTF8_ENCODER.encode(source);
  const clamped = Math.max(0, Math.min(byteOffset, bytes.length));
  return UTF8_DECODER.decode(bytes.subarray(0, clamped)).length;
}

// --- Headline resolution ---------------------------------------------------

// A headline line: one or more leading stars followed by whitespace or EOL
// (`* `, `** `, a bare `*`). Emphasis like `*bold*` never matches (no space
// after the stars).
const HEADLINE_RE = /^\*+(\s|$)/;
const PLANNING_LINE_RE = /^\s*(?:SCHEDULED|DEADLINE|CLOSED):/;

/**
 * The `headlineId` for `commands.setScheduled`: the UTF-16 start offset of the
 * headline line at or above the main cursor, or `null` when the cursor sits in
 * the preamble (no headline to plan). Convert with {@link jsIndexToUtf8Byte}
 * before sending to the backend.
 */
export function currentHeadlineId(state: EditorState): number | null {
  const cursorLine = state.doc.lineAt(state.selection.main.head);
  for (let n = cursorLine.number; n >= 1; n -= 1) {
    const line = state.doc.line(n);
    if (HEADLINE_RE.test(line.text)) return line.from;
  }
  return null;
}

// Delimiter-anchored date/time extractor for PRE-FILL ONLY (display), mirroring
// Story 4.3d's `DATE_RE`/`TIME_RE` display extraction — not a grammar re-parse.
const PREFILL_DATE_RE = /(\d{4}-\d{2}-\d{2})/;
const PREFILL_TIME_RE = /(\d{1,2}:\d{2})/;

/**
 * The value the current Headline already carries for `kind`, so the picker
 * opens on the existing date/time when modifying. `null` when there is no such
 * planning entry. This reads the source for display only; the authoritative
 * write still goes through the Rust command.
 */
export function currentPlanningValue(
  state: EditorState,
  kind: OrgPlanningKind,
): OrgDatePickerValue | null {
  const headlineId = currentHeadlineId(state);
  if (headlineId === null) return null;
  const headlineLine = state.doc.lineAt(headlineId);
  if (headlineLine.number >= state.doc.lines) return null;
  const planningLine = state.doc.line(headlineLine.number + 1);
  if (!PLANNING_LINE_RE.test(planningLine.text)) return null;

  const keyword = kind === "scheduled" ? "SCHEDULED" : "DEADLINE";
  const kwIndex = planningLine.text.indexOf(`${keyword}:`);
  if (kwIndex < 0) return null;
  // Bound the search to this keyword's stamp: stop at the next planning keyword.
  const rest = planningLine.text.slice(kwIndex + keyword.length + 1);
  const nextKw = rest.search(/(?:SCHEDULED|DEADLINE|CLOSED):/);
  const stamp = nextKw < 0 ? rest : rest.slice(0, nextKw);

  const dateMatch = PREFILL_DATE_RE.exec(stamp);
  if (dateMatch === null) return null;
  const timeMatch = PREFILL_TIME_RE.exec(stamp.slice(dateMatch.index + dateMatch[0].length));
  return { date: dateMatch[1], time: timeMatch === null ? null : timeMatch[1] };
}

// --- Applying / orchestrating the write ------------------------------------

/**
 * Apply a backend {@link PlanningEdit} as one tagged CM6 transaction. Converts
 * the UTF-8 byte offsets to UTF-16 document positions, skips genuine no-ops,
 * and never mutates mid-IME-composition (LD-6).
 */
export function applyPlanningEdit(view: EditorView, edit: PlanningEdit): void {
  if (view.composing) return;
  const source = view.state.doc.toString();
  const from = utf8ByteToJsIndex(source, edit.from);
  const to = utf8ByteToJsIndex(source, edit.to);
  if (from === to && edit.insert === "") return; // no-op (e.g. remove-when-absent)
  view.dispatch({
    changes: { from, to, insert: edit.insert },
    userEvent: PLANNING_USER_EVENT,
  });
}

/** Local calendar day as `YYYY-MM-DD` — the `today` anchor for relative
 * shortcuts resolved server-side. */
export function localTodayIso(now: Date = new Date()): string {
  const y = now.getFullYear().toString().padStart(4, "0");
  const m = (now.getMonth() + 1).toString().padStart(2, "0");
  const d = now.getDate().toString().padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/**
 * Set (`value` non-null) or remove (`value === null`) the `kind` planning
 * timestamp on the current Headline, routing the write through
 * `commands.setScheduled` and applying the returned edit. A no-op when the
 * cursor is not under a headline.
 */
export async function setPlanning(
  view: EditorView,
  kind: OrgPlanningKind,
  value: OrgDatePickerValue | null,
  today: string = localTodayIso(),
): Promise<void> {
  const source = view.state.doc.toString();
  const headlineJs = currentHeadlineId(view.state);
  if (headlineJs === null) return;
  const headlineByte = jsIndexToUtf8Byte(source, headlineJs);
  const edit = await commands.setScheduled(
    source,
    headlineByte,
    kind,
    value === null ? null : { date: value.date, time: value.time },
    today,
  );
  applyPlanningEdit(view, edit);
}

// --- Picker-open request surface (LD-26 shared, mirrors events.ts) ----------

/** A request to open the date picker for `view`, raised by the keymap. */
export interface PlanningRequested {
  kind: OrgPlanningKind;
  view: EditorView;
}

type PlanningRequestedListener = (request: PlanningRequested) => void;
const planningRequestedListeners = new Set<PlanningRequestedListener>();

/** Subscribe to picker-open requests; returns an idempotent unsubscribe. */
export function onPlanningRequested(listener: PlanningRequestedListener): () => void {
  planningRequestedListeners.add(listener);
  return () => {
    planningRequestedListeners.delete(listener);
  };
}

/** Publish a picker-open request to every current subscriber. */
export function emitPlanningRequested(request: PlanningRequested): void {
  for (const listener of [...planningRequestedListeners]) {
    try {
      listener(request);
    } catch (error) {
      // eslint-disable-next-line no-console
      console.error("onPlanningRequested listener threw", error);
    }
  }
}

/**
 * Keybindings that request the Schedule/Deadline picker for the focused view.
 *
 * Superseded as the WIRING path by Story 4.6: the native default keymap in
 * `keybindings/default.ts` is now the single source of truth and binds Schedule
 * (`Mod-Alt-s`) / Deadline (`Mod-Alt-d`) through the SAME `emitPlanningRequested`
 * surface, so behavior is identical to these interim bindings. This helper is
 * retained (and unit-tested) as a self-contained way to obtain just the planning
 * bindings; the `Editor` host no longer wires it directly. The host still decides
 * whether to honor the request (Raw mode suppresses the picker for plain typing).
 */
export function planningKeymap(): readonly KeyBinding[] {
  return [
    {
      key: "Mod-Alt-s",
      preventDefault: true,
      run: (view) => {
        emitPlanningRequested({ kind: "scheduled", view });
        return true;
      },
    },
    {
      key: "Mod-Alt-d",
      preventDefault: true,
      run: (view) => {
        emitPlanningRequested({ kind: "deadline", view });
        return true;
      },
    },
  ];
}
