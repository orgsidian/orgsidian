---
title: 'Implement Schedule/Deadline date picker'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: 'bdc1826'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** FR-9 needs friction-free Schedule/Deadline editing on the current Headline: a keyboard/context-menu action opens a date picker (calendar + time + relative shortcuts) whose confirmation writes `SCHEDULED: <YYYY-MM-DD Day HH:MM>` (or `DEADLINE:`) into the headline's planning section — while Raw mode still lets the user type the planning line by hand with no picker. Recurring timestamps (`<2026-05-19 Mon +1w>`) must survive the round-trip and stay respected by Agenda, and every other byte of the buffer must be untouched (FR-2).

**Approach:** Split the work along the epic-4-context LOCKED boundary. The **date-shortcut/timestamp backend is pure Rust** in `orgsidian-parser/src/semantic/timestamp.rs`: `resolve_date_shortcut` (`today`/`+1d`/`+1w`/`+Nm`/…), `format_planning_timestamp` (computes the weekday, re-emits carried cookies), and `set_planning_timestamp`, which returns a minimal, byte-faithful `PlanningEdit { from, to, insert }` that touches ONLY the planning line — replacing an existing same-kind stamp carries its repeater/delay cookie onto the re-picked date. The shell-app `set_scheduled` command is a thin wrapper: it resolves the wire input (a literal date or a shortcut, anchored to a frontend-supplied `today`) and widens offsets; it holds no buffer. The **frontend** is `components/org/OrgDatePicker.tsx` (calendar + `<input type=time>` + `Today`/`+1d`/`+1w`, Enter commits / Esc cancels) plus a controller `components/editor/schedule.ts` that resolves the current Headline from the CM6 selection, calls the typed `commands.setScheduled` client (never raw `invoke`), and applies the returned edit as ONE CM6 transaction tagged `input.set-planning` (LD-26 shared surface). A keybinding publishes a picker-open request that the host honors — except in Raw mode, where the AC calls for plain typing and the host suppresses the picker.

## Boundaries & Constraints

**Always:**
- Timestamp grammar (parse/format, repeaters, delays, ranges) lives in the Epic-2 semantic layer — reused, never re-implemented in TS. The one TS-side read is a delimiter-anchored date/time *extract* to pre-fill the picker on edit (display only), mirroring Story 4.3d's existing display extraction.
- Writes route through `commands.setScheduled(headlineId, timestamp)` (typed tauri-specta client) and land on the buffer as one `userEvent`-tagged CM6 transaction — the same LD-26 shared surface the checkbox/TODO widgets use. CM6 stays the sole buffer owner.
- Byte-faithful round-trip (FR-2): the returned edit's `from..to` never leaves the planning line region, so the rest of the document is untouched; recurring cookies are carried over on a re-pick.
- Offsets crossing the IPC boundary are converted between the command's UTF-8 byte offsets and CM6's UTF-16 document positions, so a headline behind non-ASCII text still edits the right bytes.
- Colors via the `--org-*` token vocabulary; strings plain English, matching the sibling Org UI Kit widgets (no lingui macros — the i18n catalog is unchanged).

**Ask First:**
- Any change to the `set_editor_mode`/`get_editor_mode` commands or Story 4.2's persistence.
- The final Schedule/Deadline chord — Story 4.6 owns the native default keymap (the epic notes an `Alt` vs `Shift` reconciliation pending); this story ships interim `Mod-Alt-s`/`Mod-Alt-d` defaults.

**Never:**
- Re-implement timestamp parsing/formatting in TS, or mutate the buffer outside the planning line.
- Open the picker in Raw mode (raw typing is the Raw-mode path).
- Reach for raw `invoke`, `plugin-fs`, or `plugin-store` — only the typed `commands.*` client.
- Mount the editor into a live route — the tests are this story's only consumer.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Add on a headline with no planning line | `* Task\nBody` | new `SCHEDULED: <…>` line inserted after the headline; body untouched | N/A |
| Add on the last headline (no trailing newline) | `* Task` (EOF) | newline + planning line appended | N/A |
| Modify an existing stamp | `SCHEDULED: <2026-05-19 Tue>` | only the `<…>` bytes change | N/A |
| Modify a recurring stamp | `SCHEDULED: <2026-05-19 Tue +1w -2d>` | date replaced, `+1w -2d` cookie preserved | N/A |
| Add a second keyword | line already has `DEADLINE:` | `SCHEDULED: <…>` appended; existing entry byte-identical | N/A |
| Remove the only entry | `SCHEDULED: <…>` alone on line | whole planning line (incl. newline) removed | N/A |
| Remove one of two | `DEADLINE: <…> SCHEDULED: <…>` | one entry + separator removed; sibling kept | N/A |
| Remove when absent | keyword not present | no-op edit (`from==to`, empty insert) | N/A |
| Relative shortcut | `date="+1w"`, `today="2026-05-19"` | resolved server-side via `resolve_date_shortcut` | malformed date → `OrgError::Parse` |
| CRLF file | `* Task\r\n…` | inserted planning line uses `\r\n` | N/A |
| Non-ASCII before headline | `* Café\n…` | edit lands on the correct bytes (UTF-8↔UTF-16 convert) | N/A |
| Raw mode keystroke | `Mod-Alt-s` in Raw | no picker; user types the line | host suppresses picker |
| Cursor not under a headline | preamble | controller no-ops (no command call) | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-parser/src/semantic/timestamp.rs` -- appended (FR-9): `PlanningKind`, `PlannedStamp`, `PlanningEdit`; `resolve_date_shortcut`, `format_planning_timestamp`, `set_planning_timestamp` (byte-faithful planning-line writer with recurring-cookie carry-over); 15 new unit tests incl. an `analyze()` re-parse round-trip.
- `crates/orgsidian-parser/src/semantic/mod.rs` -- re-export the new public items.
- `crates/orgsidian-shell-app/src/lib.rs` -- NEW command `set_scheduled` + wire types `PlanningKind`/`TimestampInput`/`PlanningEdit`; resolves shortcut-or-literal date against a frontend `today`; registered in `collect_commands!`; 4 command tests. Bindings regenerated into `shell-ui/src/lib/tauri.ts`.
- `shell-ui/src/components/org/OrgDatePicker.tsx` -- NEW. `// Implements FR-9`. Keyboard-first inline picker: month calendar, time field, `Today`/`+1d`/`+1w`; Enter commits, Esc cancels; pre-fills from an existing value.
- `shell-ui/src/components/editor/schedule.ts` -- NEW. Controller: `currentHeadlineId`/`currentPlanningValue`, UTF-8↔UTF-16 offset conversion, `applyPlanningEdit` (tagged transaction), `setPlanning` (typed-client write), and the picker-open request surface + `planningKeymap`.
- `shell-ui/src/components/editor/Editor.tsx` -- localized change: add the planning keymap to `baseEditorExtensions`, subscribe to picker-open requests (suppressed in Raw mode), render the `OrgDatePicker` overlay; data attrs moved onto the new relative wrapper (DOM contract preserved).
- `shell-ui/src/components/org/OrgDatePicker.test.tsx` + `shell-ui/src/components/editor/schedule.test.ts` -- NEW. Picker behavior + controller (offset conversion, headline resolution, pre-fill, tagged/offset-correct apply, typed-client write, keymap emit).

## Tasks & Acceptance

**Execution:**
- [x] Pure-Rust date-shortcut resolver + org-timestamp formatter (unit-tested, UI-independent).
- [x] Byte-faithful planning-line writer with recurring-cookie preservation + `analyze()` round-trip test.
- [x] `set_scheduled` tauri-specta command + regenerated bindings.
- [x] `OrgDatePicker.tsx` (calendar + time + `+1d`/`+1w`, Enter/Esc) + tests.
- [x] `schedule.ts` controller (headline resolution, offset conversion, tagged write, keymap, Raw-mode gating) + tests.
- [x] Minimal `Editor.tsx` wiring (keymap + overlay + Raw-mode suppression).

**Acceptance Criteria:**
- Given Story 4.3d, when "Set Schedule"/"Set Deadline" is invoked on the current Headline, `OrgDatePicker.tsx` opens with a calendar + time picker + `+1d`/`+1w` shortcuts — verified (`OrgDatePicker.test.tsx`, `Editor.tsx` keymap→overlay wiring).
- Confirming writes `SCHEDULED:`/`DEADLINE:` to the planning section via `commands.setScheduled` — verified (`schedule.test.ts` typed-client call; parser + command write tests).
- Recurring timestamps are preserved on round-trip and respected by Agenda — verified (`replace_carries_recurring_cookie_over`, `write_is_byte_faithful_and_reparses_via_analyze`; the preserved cookie is the same `Timestamp.repeater` Agenda already reads).
- Raw mode allows raw typing of `SCHEDULED:`/`DEADLINE:` lines without picker invocation — verified (host suppresses the picker in Raw mode; the buffer is a plain text editor there).

## Verification

**Commands:**
- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass (0 warnings).
- `cargo test --workspace --locked` — pass (268 tests; +15 parser timestamp, +4 shell-app command, export_bindings green).
- `pnpm --filter shell-ui build` — pass (tsc strict + specta regen + lingui + vite).
- `pnpm --filter shell-ui test` — 159 passed (+7 `OrgDatePicker.test.tsx`, +15 `schedule.test.ts`).
- `pnpm --filter shell-ui i18n:check` — pass (catalog unchanged).

## Design Notes

- **Offset units are the correctness hinge.** The command speaks UTF-8 byte offsets (Rust `str`); CM6 document positions are UTF-16 code units. `schedule.ts` converts both directions (`TextEncoder`/`TextDecoder`), so a headline behind `é`/emoji still edits the right bytes and the round-trip stays byte-perfect. Both endpoints always land on char boundaries (newline finds + timestamp spans), so decoding a byte prefix is always valid.
- **Cookie carry-over lives in Rust, at the write.** When replacing a same-kind stamp, `set_planning_timestamp` extracts the old stamp's repeater/delay and re-emits them onto the new date — so re-picking a date on a recurring task cannot silently drop `+1w`. The picker's `+1d`/`+1w` buttons are *calendar navigation*, distinct from a repeater cookie; the two never collide.
- **`headlineId` is a byte offset, not a persistent id.** The AST models no stable headline id, and CM6 owns the buffer. The controller resolves the current Headline from the selection (nearest `*`-line at/above the caret) and sends its byte offset; the writer re-derives the planning line from that offset. `source`/`kind`/`today` are the parameters the `setScheduled(headlineId, timestamp)` contract needs in practice — the backend holds no buffer and relative shortcuts need a reference date (supplied by the frontend, so no server clock/timezone assumption).
- **Pure-Rust resolver is load-bearing, not decorative.** `set_scheduled` runs the incoming `date` through `resolve_date_shortcut` before falling back to a literal ISO parse, so typed/raw shortcut entry (`+1w`) flows through the same tested function; the picker buttons give instant client-side feedback that agrees with it on `today`/`+1d`/`+1w`.
- **Raw-mode fallback by host gating.** The keybinding always publishes a picker-open request; the host ignores it in Raw mode. Raw mode is a plain text editor, so typing `SCHEDULED:`/`DEADLINE:` lines just works — no picker, no special path.
- **Chord is interim.** `Mod-Alt-s`/`Mod-Alt-d` are placeholders; Story 4.6 owns the native default keymap and the epic's `Alt`-vs-`Shift` reconciliation.
