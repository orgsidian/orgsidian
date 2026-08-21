---
title: 'Activate nightly memory soak gate (LD-43)'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '52f8fcd'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 4 ships the CodeMirror 6 editor surface with the full decoration/widget layer (4.1–4.4). Per Party Mode P1 (Murat), CM6 decorations are the most likely memory-leak source in the product, so NFR-21 / LD-43 — the nightly 12-hour memory soak regression gate — is anticipated from Epic 6 into this epic. `nightly.yml` currently only holds a placeholder step that `echo`s. Story 4.9 replaces that placeholder with the real dedicated Linux soak job so decoration/widget (and core-subsystem) leaks are caught within 24h of introduction, PR-blocking via the existing LD-32 stale-nightly merge gate.

**Approach:** Add a dedicated `memory-soak` job to `.github/workflows/nightly.yml` (Linux-only, `ubuntu-24.04`, `timeout-minutes: 750`). The soak driver is a standalone Rust harness at `scripts/memory-soak/` — a workspace-excluded crate (LD-5 leaf-isolation, mirroring `tools/`) that links `orgsidian-core` and runs the LD-43 scripted session against the headless core in ONE long-lived process (so `/proc/self/statm` RSS accumulates across the whole session): 200 buffer open/close cycles (`parser::analyze`), 50 plugin re-init cycles (`rebuild_index` — the drop-DB → re-designate → re-scan → shutdown restart loop), and 1000 agenda queries (`index_stats` read-path, the closest available proxy until the Epic 6/7 agenda query API lands). RSS is sampled every 30 minutes from `/proc/self/statm`; drift is computed minute-60 (warmup excluded) → minute-720 and the harness exits non-zero if growth exceeds 10%. The drift math + sample-window selection is a pure function with exhaustive unit tests. CRITICAL guard: the full 12h soak runs ONLY on the scheduled cron; `workflow_dispatch` runs a short smoke (a `soak_minutes` input, default 3) so the orchestrator's manual merge-gating nightlies validate wiring in minutes, never 12h.

## Boundaries & Constraints

**Always:**
- The soak runs in ONE process for the whole session — workloads run IN-PROCESS (linked core), so `/proc/self/statm` RSS reflects real accumulation. Shelling out per cycle would measure short-lived children and detect nothing.
- Drift is directional GROWTH: `(rss_end - rss_baseline) / rss_baseline`; a decrease never fails. Threshold is strictly `> 10%` (exactly 10% passes). Baseline = first sample at/after warmup (minute 60); end = last sample at/before the window end (minute 720).
- Hermetic env: the harness sets `ORGSIDIAN_DATA_DIR` + `XDG_CONFIG_HOME` + `XDG_DATA_HOME` to tempdirs before any core call, and synthesizes a throwaway vault — no writes to the developer's/runner's real config or data dirs (fully hermetic on Linux).
- The 12h soak runs ONLY on `github.event_name == 'schedule'`; `workflow_dispatch` runs a smoke sized by the `soak_minutes` input.
- `scripts/memory-soak` stays OUTSIDE `[workspace.members]` (root `Cargo.toml` `exclude`), like `tools/corpus-extractor` + `tools/issues-sync`.

**Ask First:**
- Any change to `pr.yml`'s `merge-gate-nightly-fresh` job — the failing-soak → blocked-merge chain rides on the EXISTING gate (a red soak makes the nightly run's conclusion `!= success`, which the gate already refuses). No change needed there.

**Never:**
- Do not make the 12h job run on manual dispatch (would block the orchestrator's merge-gating nightlies for 12h).
- Do not drive the CM6 webview here — there is no headless webview driver in the repo; the soak exercises the headless core (the closest available leak surface). The webview-level soak is a future refinement.
- Do not add `scripts/memory-soak` to the workspace (would perturb the deny.toml LEAF graph + Cargo.lock).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Scheduled nightly | `event_name == schedule` | full 12h soak: total=720min, interval=30min, warmup=60min, window-end=720min; 200 + 50 + 1000 workloads | non-zero exit on drift > 10% → nightly red → merge gate blocks |
| Manual dispatch | `workflow_dispatch`, `soak_minutes` (default 3) | smoke: total=soak_minutes, small interval, warmup=total/12; wiring validated in minutes | same exit contract, tiny window |
| Drift computation | samples with warmup spike then flat | early spike excluded (before warmup); baseline=minute-60 sample; pass | `>10%` growth → exit 1 |
| Drift exactly 10% | baseline 100MB, end 110MB | passes (strict `>`), reported | n/a |
| RSS decrease | end < baseline | passes (drift negative) | n/a |
| Too few samples | fewer than 2 usable samples | harness errors (a soak that cannot measure is a broken gate, not a skipped one) | exit non-zero with diagnostic |
| Non-Linux local smoke | macOS dev box (no `/proc`) | falls back to `getrusage(ru_maxrss)` so the harness still runs; documented approximate | n/a (CI is Linux, real `/proc/self/statm`) |

</frozen-after-approval>

## Code Map

- `.github/workflows/nightly.yml` -- replace the two `nightly memory soak (LD-43) — placeholder` steps (hosted `ubuntu-24.04` cell + arch cell) with a dedicated top-level `memory-soak` job (Linux-only, `timeout-minutes: 750`); add a `soak_minutes` `workflow_dispatch` input; the schedule-vs-dispatch guard picks 12h vs smoke params. Update the LD-43 comment block at the top (placeholder → LANDED).
- `scripts/memory-soak/Cargo.toml` -- NEW. Workspace-excluded standalone crate; `[lib]` + `[[bin]]`; path-deps `orgsidian-core`; `tokio` (rt-multi-thread), `clap`, `anyhow`, `tempfile`, `libc`.
- `scripts/memory-soak/src/lib.rs` -- NEW. `read_rss_bytes()` (`/proc/self/statm` on Linux, `getrusage` fallback elsewhere); `Sample`; `compute_drift(samples, warmup_secs, window_end_secs, threshold) -> DriftReport`; `DRIFT_THRESHOLD`; unit tests for the drift math + window selection + warmup exclusion + boundary.
- `scripts/memory-soak/src/vault.rs` -- NEW. Synthesizes a throwaway vault of varied `.org` files for the workloads.
- `scripts/memory-soak/src/main.rs` -- NEW. CLI args; hermetic env setup; initial index create; the tick loop interleaving the three workloads and sampling RSS at each 30-min mark; final drift report + exit code.
- `Cargo.toml` (workspace root) -- add `scripts/memory-soak` to `exclude`.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` -- `4-9-...` `backlog` → `review`.

## Tasks & Acceptance

**Execution:**
- [x] `scripts/memory-soak` crate: drift math (unit-tested), RSS sampler, synthetic vault, workload loop, CLI.
- [x] `nightly.yml`: dedicated `memory-soak` job replacing both placeholders; `soak_minutes` dispatch input; schedule-vs-dispatch guard; `timeout-minutes: 750`; LD-43 comment block updated.
- [x] Root `Cargo.toml` exclude + `sprint-status.yaml` transition.

**Acceptance Criteria:**
- Given 4.3 + 4.4, `nightly.yml` adds a DEDICATED Linux runner job running a 12-hour scripted session (200 buffer open/close + 50 plugin re-init + 1000 agenda queries) — verified: `memory-soak` job, `--total-minutes 720`, workload counts 200/50/1000 on the scheduled path.
- RSS sampled every 30 minutes via `/proc/self/statm` — verified: `--sample-interval-seconds 1800`; `read_rss_bytes()` parses `/proc/self/statm` field 2 × page size.
- Job fails if RSS drift > 10% over 11 hours (warmup excluded, minute 60 → 720) — verified: `compute_drift` unit tests (growth >10% fails, <10%/decrease/exactly-10% pass, warmup spike excluded).
- Failing soak blocks all PR merges to `main` — verified by inspection: a red soak sets the nightly run conclusion `!= success`; `pr.yml` `merge-gate-nightly-fresh` already refuses to merge unless the most-recent nightly is `success` within 24h. No change to that job required.

## Verification

**Commands:**
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/nightly.yml'))"` — pass (YAML valid).
- `cargo fmt --manifest-path scripts/memory-soak/Cargo.toml --all -- --check` — pass.
- `cargo clippy --manifest-path scripts/memory-soak/Cargo.toml --all-targets --locked -- -D warnings` — pass.
- `cargo test --manifest-path scripts/memory-soak/Cargo.toml --locked` — pass (drift-math unit tests).
- `cargo fmt --all -- --check` / `cargo clippy --workspace ...` / `cargo test --workspace --locked` — pass (no workspace crate changed).
- SMOKE run locally: `cargo run --manifest-path scripts/memory-soak/Cargo.toml -- --total-minutes <small> --sample-interval-seconds <small>` — reports drift, exits 0.

## Design Notes

- **Why link the core, not shell out:** RSS from `/proc/self/statm` is the harness process's OWN resident set. Leaks only accumulate if the workloads run in the same long-lived process, so the harness links `orgsidian-core` and calls `parser::analyze` / `rebuild_index` / `index_stats` in-process. Per-cycle subprocesses would each start clean and detect nothing.
- **Workload fidelity vs. what exists:** LD-43's "1000 agenda queries with varied filters" targets an agenda query API that doesn't exist until Epic 6/7. `index_stats` (the read-path aggregate over the SQLite index) is the closest available proxy and still exercises the deadpool reader pool + SQLite read path each call. Documented so a later story can swap in the real agenda query API without re-touching the CI wiring.
- **Schedule-vs-dispatch guard:** the merge gate (LD-32) is tripped by manual `workflow_dispatch` nightlies the orchestrator fires to refresh the 24h freshness window. If the soak ran 12h on dispatch, every merge would stall 12h. The job therefore branches on `github.event_name`: `schedule` → 720min/30min/60min; `workflow_dispatch` → `soak_minutes` (default 3) with a small interval. Both paths run the SAME harness and the SAME exit contract, so a dispatch smoke genuinely validates the gate end-to-end.
- **Failing-soak → blocked-merge chain:** the soak is a step in the nightly run; a non-zero harness exit fails the `memory-soak` job → the whole nightly run's conclusion is `failure`. `pr.yml`'s `merge-gate-nightly-fresh` queries the most-recent `nightly.yml` run on `main` and refuses the merge unless `conclusion == success` within 24h. So the existing LD-32 mechanism carries the AC with no change to `pr.yml`.
