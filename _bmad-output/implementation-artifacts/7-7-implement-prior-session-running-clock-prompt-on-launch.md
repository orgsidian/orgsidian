---
title: 'Story 7.7 — Implement prior-session running-clock prompt on launch'
type: 'feature'
created: '2026-09-14'
status: 'done'
review_loop_iteration: 0
baseline_commit: 'ff1b831d0e92c6863f8b75aad356e79570e3c7f7'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** A clock left running when Orgsidian closed (UJ-1 edge case) would, on relaunch, silently keep counting wall-clock time — recording e.g. 14 h from a laptop left open overnight. Story 7.6 persists the running clock (`active-clock.json` `{headline_id, started_at, last_active_at}`) and an open `CLOCK:` line, but nothing surfaces or reconciles it at launch. Story 7.6 also deferred to this story the guard against a second open `CLOCK:` line when the pointer was lost.

**Approach:** On launch, when a prior-session Active Clock exists, show a modal naming the tracked Headline, its `last_active_at`, and both candidate durations (keep-tracking = now − started; adjust = last_active − started), offering three actions in safest-default order: **Adjust end time** (default), **Keep tracking**, **Discard this session**. Back these with a computed launch summary and two new clock transitions in `orgsidian-core::clock` (discard the open line; close at an explicit end time), reusing the existing `clock_out`/`clock_resume` primitives. Add the deferred `clock_in` guard so clocking into a Headline that already has an open line adopts it (and neutralizes duplicate open lines) instead of orphaning it.

## Boundaries & Constraints

**Always:**
- Reuse Story 7.6 primitives — extend `orgsidian-core::clock` and the shell command surface; never rewrite them. All `CLOCK:` mutations stay byte-faithful text splices + `atomic_write` (never a serializer re-render); open lines are located by matching `started_at`, never a stale byte offset.
- Wall-clock `now` stays dependency-injected into core (`NaiveDateTime`); only the shell command layer reads `chrono::Local::now().naive_local()` via the existing `now_naive()`.
- New index items are ADDITIVE free functions only — do NOT touch the frozen `IndexQuery` trait / `AgendaItem` (Story 6.5 semver gate).
- Safest default: the modal's default-focused / Enter action is **Adjust end time**; **Esc invokes Adjust** (not cancel/close); "Adjust" pre-fills its time picker from `last_active_at`.
- "Keep tracking" keeps `started_at` and performs no source mutation. "Discard" removes the open `CLOCK:` line from the source LOGBOOK. "Adjust" closes the open line to the user-chosen end (`clock_out` clamps a too-early end to `=> 0:00`).
- Follow conventions: commands are `#[tauri::command] #[specta::specta]` fns returning `OrgResult<T>`, registered in `collect_commands!`; DTOs carry `#[serde(rename_all = "camelCase")]`; wire ints narrowed to `u32`. Frontend calls `commands.*` from `@/lib/tauri` (Throw mode → `Promise<T>`, throws `{kind, reason}`); modal uses `ui/dialog.tsx` (controlled `open`/`onOpenChange`); tests hand-roll `createRoot`+`act` and `vi.mock("@/lib/tauri")`.
- Record the final modal microcopy in `docs/microcopy-registry.md` as a `[draft]` Story 7.7 entry.

**Never:**
- Never mutate `IndexQuery`/`AgendaItem`; never re-render CLOCK text through the serializer; never read the wall clock inside core.
- Never push, open PRs, run network/`gh`, or edit `sprint-status.yaml`. No new dependencies.
- Out of scope: status-bar Active Clock indicator (Epic 13), the ClockEditor time-entry UI (Story 7.8), whole-vault sweeps for orphaned open lines in files with no pointer (reconciliation here is pointer-driven + the `clock_in` guard on the target Headline only).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Launch, no active clock | `active-clock.json` absent | `get_stale_clock` → `None`; no modal | N/A |
| Launch, active clock | pointer + open line, 14 h gap | `get_stale_clock` → summary {headline, started_at, last_active_at, keepDuration=now−started, adjustDuration=last_active−started} | pointer present but Headline/line desynced → `OrgError::Vault` |
| Keep tracking | modal, pointer present | `clock_resume(headline_id)` re-adopts open line; `started_at` unchanged; source unchanged | as `clock_resume` |
| Discard this session | modal, pointer present | open `CLOCK:` line removed from LOGBOOK; pointer cleared | no matching open line → clear pointer, `OrgError::Vault` |
| Adjust end time | end = chosen datetime string | open line closed `[start]--[end] => H:MM`; pointer cleared | end < start → duration clamps `=> 0:00`; unparseable end → `OrgError::Vault` |
| clock_in guard, pre-existing open line | target Headline already has 1 open line, pointer lost | adopt it (write pointer to its start); no second open line inserted | N/A |
| clock_in guard, >1 open line | target Headline has ≥2 open lines | adopt most-recent; neutralize older ones (close each to its own start `=> 0:00`) | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-index/src/query/locate.rs` — ADD additive `pub fn title(conn, id) -> Result<Option<String>, IndexError>` (`SELECT title FROM headlines WHERE id=?1 AND kind='headline'`) + inline unit test. Do NOT change `HeadlineLocation`/`headline()` or the frozen `IndexQuery` trait.
- `crates/orgsidian-core/src/index/mod.rs` — ADD `pub async fn headline_title(vault_root, id) -> OrgResult<Option<String>>` fresh-pool wrapper mirroring `locate_headline` (mod.rs:318) → `orgsidian_index::query::locate::title`.
- `crates/orgsidian-core/src/clock.rs` — the extension surface (built on 7.6):
  - `StaleClockSummary { headline_id: u32, headline: String, started_at, last_active_at, keep_duration, adjust_duration }` + `pub async fn stale_clock_summary(vault_root, now) -> OrgResult<Option<StaleClockSummary>>`: read `active_clock`; `None` → `None`; else resolve title via `headline_title`, compute both durations with the existing `format_duration`/`parse_ts`/`truncate_to_minute` helpers.
  - `pub async fn clock_discard(vault_root) -> OrgResult<()>`: locate the active pointer's open line (scoped to its Headline by ordinal, matched by `started_at`, like `clock_out`); splice-delete the whole open `CLOCK:` line (`line_start`..`next_line_start`); `atomic_write`; `remove_active_clock`. Desync → clear pointer + `OrgError::Vault`.
  - MODIFY `clock_in`: after resolving the target Headline, if it already has open line(s), adopt the most-recent (write pointer to its start, no new line) and close older duplicates to their own start `=> 0:00` via a reverse-order multi-splice helper `close_extra_open_lines`. Guards the 7.6-deferred double-open-line orphan. Existing "fresh insert when no open line" path unchanged.
- `crates/orgsidian-core/src/lib.rs` — extend the `pub use clock::{...}` block with `clock_discard, stale_clock_summary, StaleClockSummary`; re-export `headline_title` if needed.
- `crates/orgsidian-shell-app/src/lib.rs` — ADD commands `get_stale_clock(state) -> Option<StaleClockDto>` (uses `now_naive()`), `clock_discard(state)`, `clock_adjust_end(end_at: String, state)` (parse `%Y-%m-%dT%H:%M:%S` → `NaiveDateTime`, else `%Y-%m-%dT%H:%M`; call `orgsidian_core::clock_out(&root, end)`); all take `state.clocking.lock().await` for mutations and `current_vault_root().ok_or_else(no_active_vault)?`. ADD `StaleClockDto` (camelCase) + `From`. Register the three in `collect_commands!` (lib.rs:901). Keep-tracking reuses existing `clock_resume`. Inline DTO unit test.
- `shell-ui/src/components/clock/StaleClockPrompt.tsx` — NEW. Controlled `ui/dialog.tsx` modal. On mount (once) calls `commands.getStaleClock()`; renders nothing while in-flight / when `null`; else opens. Three `Button`s; Adjust default-focused via `onOpenAutoFocus`; `onEscapeKeyDown` → open the Adjust time picker (prevent default close). Adjust reveals native `<input type="date">`+`<input type="time">` pre-filled from `lastActiveAt`, confirm → `commands.clockAdjustEnd(iso)`. Keep → `commands.clockResume(headlineId)`. Discard → `commands.clockDiscard()`. Use the `errorMessage` helper; close on success.
- `shell-ui/src/routes/_layout/today.tsx` — mount `<StaleClockPrompt />` when `vaultConfigured === true` (alongside the existing dashboard branch).
- `shell-ui/src/components/clock/StaleClockPrompt.test.tsx` — NEW. `vi.mock("@/lib/tauri")`; re-declare `StaleClockDto` locally; assert: no clock → no dialog; summary → dialog with copy + focused Adjust; each button calls the right command; Esc opens the picker.
- `crates/orgsidian-core/tests/stale_clock.rs` — NEW integration test (copy the `tests/clock.rs` `scanned_vault`/`headline_id_by_title` harness): seed a 14 h gap (started 04:00, last_active 18:00, now next-day), assert each transition — Keep (source unchanged, pointer kept), Discard (open line gone, pointer cleared), Adjust (line closed `=> 14:00`, pointer cleared) — plus the `clock_in` guard (no second open line; duplicates neutralized).
- `docs/microcopy-registry.md` — ADD the Story 7.7 `[draft]` modal-copy entry.

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-index/src/query/locate.rs` — add `title(conn, id)` + unit test.
- [x] `crates/orgsidian-core/src/index/mod.rs` — add `headline_title` wrapper.
- [x] `crates/orgsidian-core/src/clock.rs` — add `StaleClockSummary`+`stale_clock_summary`, `clock_discard`, `close_extra_open_lines`; add the `clock_in` open-line-adopt guard. Inline `#[cfg(test)]` unit tests for line-delete splice, duplicate neutralization, and summary duration math.
- [x] `crates/orgsidian-core/src/lib.rs` — extend re-exports.
- [x] `crates/orgsidian-shell-app/src/lib.rs` — add `get_stale_clock`/`clock_discard`/`clock_adjust_end` + `StaleClockDto`, register in `collect_commands!`; DTO unit test.
- [x] `shell-ui/src/components/clock/StaleClockPrompt.tsx` (+ `.test.tsx`) — build the modal.
- [x] `shell-ui/src/routes/_layout/today.tsx` — mount the prompt.
- [x] `crates/orgsidian-core/tests/stale_clock.rs` — 14 h-gap integration test per AC.
- [x] `docs/microcopy-registry.md` — record the draft copy.

**Acceptance Criteria:**
- Given a prior-session running clock in `active-clock.json`, when Orgsidian launches, then a modal prompts with the tracked Headline, formatted `last_active_at`, and both keep/adjust durations, offering `[Adjust end time]` `[Keep tracking]` `[Discard this session]`.
- Given the modal, then the keyboard default-focused button is "Adjust end time"; Enter confirms it; Esc invokes Adjust (not cancel); Adjust opens a time picker pre-filled with `last_active_at`.
- Given "Keep tracking", when chosen, then the clock resumes from the original `started_at` with no source mutation.
- Given "Discard this session", when chosen, then the open `CLOCK:` line is removed from the source LOGBOOK drawer and the pointer cleared.
- Given a Headline that already has an open `CLOCK:` line but no pointer, when `clock_in` runs on it, then it is adopted (no second open line); with >1 open line, duplicates are neutralized to `=> 0:00`.
- Given `crates/orgsidian-core/tests/stale_clock.rs` faking a 14 h `started_at`→`last_active_at` gap, then each button's documented state transition is asserted.
- Given the frozen index API, then `IndexQuery`/`AgendaItem` are unchanged (cargo-semver-checks stays green).

## Design Notes

Durations use 7.6's `format_duration` (`H:MM`, negative clamps `0:00`). "Adjust" end datetime is built frontend-side as `${date}T${time}:00` from the pre-filled `last_active_at`, so a plain time edit stays on the last-active day while a cross-midnight case is still expressible via the date input.

`clock_discard` line delete: `let ls = line_start(&source, entry.span.start); let le = next_line_start(&source, entry.span.start); splice(&source, ls, le, "")` — removes exactly the open line (with its newline), leaving an empty `:LOGBOOK:`/`:END:` drawer (valid org), matching "remove the open CLOCK line" literally.

`clock_in` guard: collect the target's open entries; if non-empty, adopt `most_recent_open_clock` (its `clock_start_dt` → pointer), and for every OTHER open entry splice `--[start] => 0:00` at its `span.end` — apply these splices in descending `span.end` order so earlier offsets stay valid, then a single `atomic_write`. No 7.6 test clocks into a Headline that already owns an open line, so existing behavior is preserved.

## Verification

**Commands:**
- `cargo test -p orgsidian-core` — unit + `tests/clock.rs` + `tests/stale_clock.rs` pass.
- `cargo test -p orgsidian-index -p orgsidian-shell-app` — locate `title` test + DTO tests pass.
- `cargo fmt --all -- --check` — clean.
- `cargo clippy -p orgsidian-core -p orgsidian-index -p orgsidian-shell-app --all-targets` — no new warnings.
- `cargo test -p orgsidian-shell-app --test export_bindings` then `pnpm --filter shell-ui exec tsc --noEmit` — bindings regenerated; `getStaleClock/clockDiscard/clockAdjustEnd` present; TS clean.
- `pnpm --filter shell-ui exec vitest run` — `StaleClockPrompt.test.tsx` green.
</content>
</invoke>

## Suggested Review Order

**Launch prompt (entry point)**

- Where the prompt mounts — the first configured-Vault screen, self-gating on a stale clock.
  [`today.tsx:75`](../../shell-ui/src/routes/_layout/today.tsx#L75)

- The modal: once-per-mount `getStaleClock`, safest-default focus, Esc→Adjust, three actions.
  [`StaleClockPrompt.tsx:70`](../../shell-ui/src/components/clock/StaleClockPrompt.tsx#L70)

- Default-focus Adjust and route Esc to Adjust (no dismiss-without-choosing path).
  [`StaleClockPrompt.tsx:152`](../../shell-ui/src/components/clock/StaleClockPrompt.tsx#L152)

- The command feeding the modal; injects wall-clock `now` for the two durations.
  [`lib.rs:918`](../../crates/orgsidian-shell-app/src/lib.rs#L918)

- Core summary: title + normalized timestamps + keep/adjust durations; self-heals a malformed `last_active_at`.
  [`clock.rs:779`](../../crates/orgsidian-core/src/clock.rs#L779)

**Reconciliation transitions**

- Discard: delete the whole open `CLOCK:` line (matched by `started_at`), clear the pointer.
  [`clock.rs:841`](../../crates/orgsidian-core/src/clock.rs#L841)

- Adjust: parse the picker's ISO end, reuse `clock_out` (too-early clamps `=> 0:00`).
  [`lib.rs:957`](../../crates/orgsidian-shell-app/src/lib.rs#L957)

- Extracted, unit-tested end-time parser (dual ISO forms → `OrgError::Vault`).
  [`lib.rs:942`](../../crates/orgsidian-shell-app/src/lib.rs#L942)

**Pre-existing open-line reconciliation (7.6-deferred guard)**

- `clock_in` now adopts an existing open line instead of orphaning it with a second.
  [`clock.rs:527`](../../crates/orgsidian-core/src/clock.rs#L527)

- Neutralize older duplicate open lines to `=> 0:00` via a reverse-order multi-splice.
  [`clock.rs:412`](../../crates/orgsidian-core/src/clock.rs#L412)

**Headline resolution (additive index surface — frozen IndexQuery untouched)**

- New additive query: headline title by rowid.
  [`locate.rs:90`](../../crates/orgsidian-index/src/query/locate.rs#L90)

- Core fresh-pool wrapper the summary calls.
  [`index/mod.rs:345`](../../crates/orgsidian-core/src/index/mod.rs#L345)

**Tests, wire types & docs (peripherals)**

- 14 h-gap integration tests: each button's transition + discard/summary desync branches.
  [`stale_clock.rs:1`](../../crates/orgsidian-core/tests/stale_clock.rs#L1)

- Frontend tests incl. the rejected-command "error shown, dialog stays open" case.
  [`StaleClockPrompt.test.tsx:1`](../../shell-ui/src/components/clock/StaleClockPrompt.test.tsx#L1)

- IPC DTO (camelCase) distinct from the snake_case sidecar.
  [`lib.rs:884`](../../crates/orgsidian-shell-app/src/lib.rs#L884)

- Draft modal microcopy recorded for the content pass.
  [`microcopy-registry.md`](../../docs/microcopy-registry.md)
