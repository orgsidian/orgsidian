---
title: 'Story 7.8 — Implement ClockEditor component for time entry editing'
type: 'feature'
created: '2026-09-14'
status: 'in-review'
review_loop_iteration: 0
baseline_commit: '6476302d9c45a636195c99cc2ae16459fe2b48cb'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 7.6 persists `CLOCK:` entries in the LOGBOOK drawer and 7.7 reconciles a running clock at launch, but there is no way to correct an already-recorded time entry (forgot-to-clock-out, wrong day). FR-8 needs an in-app editor for a closed `CLOCK: [start]--[end] => HH:MM` line.

**Approach:** Add a core `update_clock_entry` transition that byte-faithfully rewrites one closed entry's start/end and recomputes its `=> HH:MM` duration (source of truth = the org file), exposed over a new `commands.updateClockEntry(headlineId, entryIndex, newStart, newEnd)` Tauri command. Build a props-driven `ClockEditor.tsx` dialog whose start / end / duration fields are interdependent (edit start or end → recompute duration; edit duration → recompute end), validated (end ≥ start, minute precision) before confirm, reusing 7.7's `Dialog` + native date/time-input conventions.

## Boundaries & Constraints

**Always:** Locate the target Headline by stable document-order ordinal, then the target entry by its 0-based index into `headline.clocks` (document order) — never a stale byte offset. Rewrite via `splice` over the entry's full `span` + `atomic_write` (FR-2 round-trip: every other byte identical). Serialize the mutation under `AppState.clocking`. Truncate both stamps to whole minutes; recompute duration authoritatively from `end - start` via `format_duration`. Reuse existing primitives (`find_headline_by_ordinal`, `format_inactive_stamp`, `format_duration`, `splice`, `atomic_write`, `locate_headline`).

**Ask First:** Building a full LOGBOOK drawer surface or a query command that lists a Headline's entries (neither exists yet; both are out of scope here).

**Never:** No new persistence deps. Do not edit a RUNNING (open, `end == None`) entry — reject it (the running clock is corrected via 7.7's adjust-end flow); this keeps the `active-clock.json` pointer invariant intact. Do not re-render through the parser/serializer. Do not change the frozen `IndexQuery` trait. Do not touch `sprint-status.yaml`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Edit closed entry | headline_id, entry_index of a closed line, new_start < new_end | Line rewritten `CLOCK: [new_start]--[new_end] => H:MM`; rest of file byte-identical; `Ok(())` | N/A |
| Duration recompute | new_start=10:00, new_end=11:30 | Written suffix `=> 1:30` (minute-truncated, negative clamps 0:00) | N/A |
| End before start | new_end < new_start | No write | `OrgError::Vault` (end is before start) |
| Index out of range | entry_index ≥ `clocks.len()` | No write | `OrgError::Vault` (index out of range) |
| Running entry | target entry has `end == None` | No write | `OrgError::Vault` (cannot edit a running entry) |
| Headline gone | ordinal no longer resolves | No write | `OrgError::Vault` (headline not found) |
| Unparseable stamp | new_start/new_end not `%Y-%m-%dT%H:%M[:%S]` | No write | `OrgError::Vault` (unparseable) |
| No vault | no active vault | No write | `OrgError::Vault` (no active vault) |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-core/src/clock.rs` — ADD `pub async fn update_clock_entry(vault_root, headline_id: u32, entry_index: usize, new_start: NaiveDateTime, new_end: NaiveDateTime) -> OrgResult<()>`. Mirror `clock_out` (clock.rs:582) end-to-end: `truncate_to_minute` both; reject `new_end < new_start`; `locate_headline` → `read_source` → `analyze_source` → `find_headline_by_ordinal` (clock.rs:275); `headline.clocks.get(entry_index)` (reject out-of-range and `entry.end.is_none()`); build `format!("CLOCK: {start}--{end} => {dur}")` from `format_inactive_stamp` (clock.rs:182) + `format_duration(new_end - new_start)` (clock.rs:189); `splice(&source, entry.span.start, entry.span.end, &new_line)` (clock.rs:202) — span is line content after indent, no trailing newline (confirmed vs `clock_out` splice at `span.end`); `atomic_write`. Inline `#[cfg(test)]` unit tests for the pure line-shape/duration/clamp cases.
- `crates/orgsidian-core/src/lib.rs:87` — extend `pub use clock::{...}` with `update_clock_entry`.
- `crates/orgsidian-shell-app/src/lib.rs` — ADD `#[tauri::command] async fn update_clock_entry(headline_id: u32, entry_index: u32, new_start: String, new_end: String, state)`: `current_vault_root().ok_or_else(no_active_vault)?`; parse both stamps (generalize `parse_adjust_end`, lib.rs:971, into `parse_wire_datetime` accepting `%Y-%m-%dT%H:%M:%S` then `%Y-%m-%dT%H:%M`); `let _clocking = state.clocking.lock().await;`; call core with `entry_index as usize`. Register in `collect_commands!` (lib.rs:1019, after `clock_adjust_end`). Inline parse unit test.
- `shell-ui/src/lib/tauri.ts` — GENERATED (git-ignored). Regenerate via `cargo test -p orgsidian-shell-app --test export_bindings` after registering the command; surfaces `commands.updateClockEntry`.
- `shell-ui/src/components/org/ClockEditor.tsx` — NEW. Props `{ headlineId, entryIndex, initialStart, initialEnd, open, onOpenChange, onSaved? }` (ISO `YYYY-MM-DDTHH:MM`). Controlled shadcn `Dialog` (mirror `StaleClockPrompt.tsx`); native `<input type="date">`+`<input type="time">` for start and end; a duration `H:MM` text input. Interdependence: editing start/end recomputes duration; editing duration recomputes end (= start + duration). Confirm disabled while invalid (end < start / unparseable); confirm → `commands.updateClockEntry(headlineId, entryIndex, `${startDate}T${startTime}:00`, `${endDate}T${endTime}:00`)` → `onSaved?.()` + close; error → `errorMessage(err)`.
- `shell-ui/src/components/org/ClockEditor.test.tsx` — NEW. `createRoot`+`act` (mirror `StaleClockPrompt.test.tsx`); `vi.mock("@/lib/tauri")`. Assert: fields pre-fill; edit end → duration recomputes; edit duration → end recomputes; end<start disables/blocks confirm; confirm calls `updateClockEntry` with the right ISO args; error surfaces.
- `crates/orgsidian-core/tests/update_clock_entry.rs` — NEW integration test (copy `tests/clock.rs` `scanned_vault`/`headline_id_by_title` harness). Seed a headline with a closed `CLOCK:` line; edit start/end; assert the rewritten stamps + recomputed duration and byte-faithful remainder; assert the open-entry and out-of-range rejections.

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-core/src/clock.rs` -- add `update_clock_entry` + inline unit tests -- the core byte-faithful rewrite transition.
- [x] `crates/orgsidian-core/src/lib.rs` -- re-export `update_clock_entry`.
- [x] `crates/orgsidian-shell-app/src/lib.rs` -- add the `update_clock_entry` command + `parse_wire_datetime` + register in `collect_commands!` + parse unit test.
- [x] regenerate bindings -- `cargo test -p orgsidian-shell-app --test export_bindings`.
- [x] `shell-ui/src/components/org/ClockEditor.tsx` (+ `.test.tsx`) -- build the interdependent-fields dialog.
- [x] `crates/orgsidian-core/tests/update_clock_entry.rs` -- end-to-end edit over a scanned vault (+ cross-midnight, and the end-before-start / out-of-range / running / vanished-ordinal rejections).

**Acceptance Criteria:**
- Given a closed `CLOCK:` entry, when `ClockEditor` opens, then start / end / duration fields show its values and editing any field recomputes the others (duration = end − start; editing duration recomputes end).
- Given valid edits (end ≥ start), when confirmed, then `commands.updateClockEntry(headlineId, entryIndex, newStart, newEnd)` rewrites the `CLOCK:` line with the new stamps and recomputed `=> HH:MM`, every other byte identical, and `onSaved` fires so the caller re-renders.
- Given end < start or an out-of-range / running entry, then the core rejects with `OrgError::Vault` and the UI blocks confirm.
- Given the frozen index API, then `IndexQuery`/`AgendaItem` are unchanged.

## Design Notes

Span rewrite: `ClockEntry.span` is `line_offset + indent .. line_offset + cursor` (drawer.rs) — i.e. the `CLOCK: …` content without the leading indent and without the trailing newline (this is why `clock_out` appends `--[end] => dur` at `span.end`). So `splice(source, span.start, span.end, "CLOCK: [s]--[e] => H:MM")` preserves the drawer indent (before `span.start`) and the newline (after `span.end`) untouched.

Frontend ISO shape matches 7.7: `${date}T${time}:00`; the command accepts both the `:%S` and no-seconds forms via `parse_wire_datetime`. Duration edit parses `H:MM` → minutes; `end = start + minutes`; a blank/invalid duration is a validation error, not a silent 0.

## Verification

**Commands:**
- `cargo fmt --all -- --check` -- expected: clean.
- `cargo test -p orgsidian-core -p orgsidian-index -p orgsidian-shell-app` -- expected: unit + `tests/update_clock_entry.rs` + parse test pass.
- `cargo clippy -p orgsidian-core -p orgsidian-index -p orgsidian-shell-app --all-targets` -- expected: no new warnings.
- `cargo test -p orgsidian-shell-app --test export_bindings` then `pnpm --filter shell-ui exec tsc --noEmit` -- expected: `updateClockEntry` present in `tauri.ts`; TS clean.
- `pnpm --filter shell-ui exec vitest run` -- expected: `ClockEditor.test.tsx` green.
</content>
</invoke>
