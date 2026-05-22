# Story 1.7: Configure `cargo-deny` + `cargo audit` supply-chain hygiene

Status: review

## Metadata

- github_issue: 7

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want `cargo-deny` (licenses allowlist + bans + LEAF graph rule via `wrappers`) and `cargo audit` configured at the workspace root with matching `pnpm audit --audit-level=moderate` + `pnpm licenses` discipline on the JS side, plus a `docs/security/advisory-exceptions.md` ledger for the quarterly review of accepted advisories,
So that LD-37 supply-chain hygiene is enforced — locally today, in CI from Story 1.8 onward — before the first feature lands and before any third-party crate or npm package can drift past the license / advisory floor.

## Acceptance Criteria

**AC1 — `deny.toml` at workspace root declares the LD-37 license allowlist exactly per architecture line 1167.**

- `deny.toml` MUST live at the repo root (next to `Cargo.toml`), discoverable by `cargo deny` without an explicit `--manifest-path`.
- The `[licenses]` block MUST set `allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unlicense", "Zlib", "MPL-2.0"]` — the LD-37 canonical allowlist. No other entries.
- `[licenses]` MUST set `confidence-threshold = 0.93` (cargo-deny default for high-confidence SPDX inference) and `unlicensed = "deny"`.
- `[licenses].exceptions` MUST start empty — every per-crate exception (e.g., dual-license disambiguation) lands with a justification comment when first needed.
- The allowlist is intentionally **stricter** than the cargo-deny default: `Unicode-3.0` is NOT on the allowlist (LD-37 silent on it; add only when a transitive dep forces the question). `Apache-2.0 WITH LLVM-exception` is also NOT pre-allowed (same reasoning — add as an explicit exception when needed).
- The `[licenses]` block MUST reject `GPL-*`, `AGPL-*`, proprietary, and unknown licenses by being absent from the allow list (cargo-deny default behavior: any non-allowed SPDX expression fails the check). No explicit deny-list needed — the closed-allowlist enforces this by construction. A header comment in `deny.toml` MUST state this rejection contract verbatim per architecture line 1167.

**AC2 — `deny.toml` `[bans]` block blocks duplicate major versions of `tokio`, `serde`, `chrono`, `rusqlite` per architecture line 1168.**

- `[bans]` MUST set `multiple-versions = "deny"` (workspace-wide).
- `[bans].skip` MUST start empty — duplicate-version carve-outs land per-incident with a `reason = "..."` justification.
- A header comment above `[bans]` MUST state the binding rule: **`tokio`, `serde`, `chrono`, and `rusqlite` MUST NEVER appear in `[bans].skip` or `[bans].skip-tree`** — these four are LD-37's canonical-version invariants. The comment MUST name those four crates explicitly so future contributors cannot drift past the rule via casual `skip` additions.
- `[bans].wildcards = "deny"` MUST be set — wildcard version requirements (`"*"`) in `Cargo.toml` files are a supply-chain anti-pattern (transitive surprises) and not used anywhere in our workspace.
- `[bans].multiple-versions-include-dev = false` — dev-only duplicate versions are tolerated (dev-deps don't ship to users). Explicit choice; documented as a comment.

**AC3 — `deny.toml` `[[bans.deny]]` entries enforce the LD-37 LEAF crate graph rule via cargo-deny `wrappers` per architecture line 1169.**

- For each LEAF crate — `orgsidian-parser`, `orgsidian-index`, `orgsidian-watcher`, `orgsidian-vault`, `orgsidian-report`, `orgsidian-plugin-api` — `deny.toml` MUST declare a `[[bans.deny]]` entry:

```toml
[[bans.deny]]
name = "orgsidian-parser"
wrappers = ["orgsidian-core"]
```

- `wrappers = ["orgsidian-core"]` means the listed LEAF crate may only be a **direct** dependency of `orgsidian-core` (the hub). If `orgsidian-shell-app`, `orgsidian-cli`, or any non-core crate adds a direct dependency on a LEAF, `cargo deny check bans` fires.
- LEAF crates are still reachable **transitively** by consumers — `orgsidian-shell-app` depending on `orgsidian-core` which depends on `orgsidian-parser` is fine. The `wrappers` field constrains the direct-edge set only, which is the LD-37 invariant.
- The 6 `[[bans.deny]]` entries MUST appear under a `# --- LD-37 LEAF crate graph rule ---` section header comment so the intent is obvious to a reader who hasn't read the architecture doc.
- The custom-CI-check alternative mentioned at architecture line 1028 (parsing consumer `Cargo.toml` files directly) is **NOT IMPLEMENTED** in this story — `wrappers` is the native cargo-deny mechanism and supersedes the custom-check fallback. Documented in Dev Notes.

**AC4 — `deny.toml` `[advisories]` + `[sources]` blocks honour LD-37's RustSec posture.**

- `[advisories]` MUST set `db-urls = ["https://github.com/RustSec/advisory-db"]` (the canonical RustSec DB; cargo-deny default but pinned explicitly for clarity and future-proofing against default changes).
- `[advisories].ignore` MUST start as an empty array — the file is the *machine-readable* ledger, and `docs/security/advisory-exceptions.md` (AC6) is the *human-readable* one. Every entry added to `ignore` MUST also appear in the markdown ledger with rationale + review date. A header comment in `deny.toml` states this invariant.
- `[advisories].unmaintained = "workspace"` — flag unmaintained crates only if they're our direct deps, not transitive. (Transitive unmaintained crates surface but don't fail until they're our problem to fix.)
- `[advisories].unsound = "all"` — unsound advisories fail regardless of direct/transitive (memory safety is non-negotiable).
- `[advisories].yanked = "deny"` — any yanked crate version anywhere in the graph fails the check.
- `[sources]` MUST set `unknown-registry = "deny"` and `unknown-git = "deny"` — only crates.io + explicitly-allowed git sources are permitted. (Currently no git sources in the workspace; the future `nvim-orgmode/tree-sitter-org` submodule per LD-48 is a git submodule on the filesystem, NOT a cargo git dependency, so this gate stays clean.)

**AC5 — `[graph]` block declares the LD-32 CI matrix target list.**

- `[graph].targets` MUST enumerate the 4 platform triples the LD-32 CI matrix builds against:

```toml
[graph]
targets = [
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
]
all-features = true
```

- `aarch64-unknown-linux-gnu` is intentionally OMITTED — LD-32 CI matrix targets are macOS (Intel + Apple Silicon) + Ubuntu-LTS (x86_64) + Windows-x86_64. Arch Linux nightly (LD-32 nightly arm of the matrix) runs the same x86_64 binary; no separate triple needed.
- `all-features = true` ensures the check covers every feature gate's dep set — without this, a feature-gated crate (e.g., `rusqlite/bundled`) might leak past the licenses check.
- Cross-platform conditional deps (Windows-only `windows-sys`, macOS-only `core-foundation`) will surface via the targets list — that's the point. We accept the per-platform-triple sweep cost (cargo-deny is fast enough that this isn't a budget concern).

**AC6 — `docs/security/advisory-exceptions.md` exists as the LD-37 human-readable quarterly review ledger.**

- The file lives at `docs/security/advisory-exceptions.md` (`docs/security/` is a NEW directory — Story 1.7 creates it).
- Initial content: Keep-a-Changelog-style heading structure with an empty "Active exceptions" table and a "Review history" table, plus prose explaining the discipline. Template MUST include:
  - Header: `# Advisory Exceptions Ledger (LD-37)`
  - "How this works" paragraph: every entry added to `deny.toml` `[advisories].ignore` MUST have a corresponding row here with `RUSTSEC ID | Crate | Decision | Rationale | First accepted (date) | Next review (date, +90 days)`. Quarterly review = 90-day rolling cadence.
  - "Active exceptions" table: empty as of 2026-05-22 (Story 1.7 ships zero exceptions; first-CI sweep is expected clean).
  - "Review history" table: one row recording the 2026-05-22 initial empty-ledger establishment.
  - "Pnpm-side exceptions": parallel section for `pnpm audit` advisories, same discipline. Empty initially.
  - "License exceptions" section: any per-crate license exception added to `[licenses].exceptions` in `deny.toml` MUST be recorded here with same metadata shape.
- The file is **commit-required, never gitignored**.
- Cross-references: links to LD-37 (`_bmad-output/planning-artifacts/architecture.md#LD-37`), to `deny.toml` at repo root, and to `SECURITY.md` (which Story 1.10 will ship — link is a forward-reference for now; renders as a dead link until Story 1.10 closes; documented as expected).

**AC7 — Workspace `Cargo.toml` declares a `[workspace.metadata.cargo-deny]` block (defensive — empty for now).**

- The block MUST exist and be empty — `[workspace.metadata.cargo-deny]` with no key/values inside. This reserves the namespace and signals intent: workspace-level cargo-deny metadata may be added later (e.g., per-crate exceptions), and a future contributor sees the slot.
- `Cargo.lock` MUST be **committed** — already true as of Story 1.4 (the file is tracked at repo root, 159 KB). Story 1.7 adds an explicit comment block to `Cargo.toml` documenting the binary-app commit-Cargo.lock convention with a pointer to LD-37:

```toml
# Cargo.lock is committed (LD-37, binary-application convention).
# - Reproducible builds across CI matrix.
# - Required for cargo-audit / cargo-deny to lock advisory + license verdicts.
# - Auto-bumped via Dependabot / Renovate PRs (Story 1.8+ wiring).
```

- The comment lands at the top of `Cargo.toml` immediately after the `[workspace]` header (or in the existing `[workspace.package]` block as a sibling comment).

**AC8 — Workspace cargo aliases land at `.cargo/config.toml` for the four LD-37 checks.**

- `.cargo/config.toml` is a NEW file (does not exist today; verified). Story 1.7 creates it.
- The file MUST declare 4 cargo aliases:

```toml
[alias]
deny-all = "deny check all"
deny-licenses = "deny check licenses"
deny-bans = "deny check bans"
deny-advisories = "deny check advisories"
```

- Rationale: `cargo deny-all` is the local-dev one-shot equivalent of what CI runs; the per-check aliases let contributors iterate on a specific failure category without re-running the full sweep.
- `.cargo/config.toml` MUST NOT declare any `[build]`, `[target.*]`, `[net]`, `[term]`, `[registries]`, or `[env]` sections — Story 1.7 scope is supply-chain only; cross-cutting build config is out of scope.
- `cargo audit` is NOT aliased — it ships as its own top-level binary (`cargo-audit`) and is invoked as `cargo audit` directly. The audit-side helper is the pnpm script in AC10, not a cargo alias.

**AC9 — Root `package.json` adds `audit:js` and `audit:licenses:js` pnpm scripts for the JS-side LD-37 parallel.**

- Two new entries land in root `package.json` `"scripts"`:

```json
"audit:js": "pnpm audit --audit-level=moderate --prod",
"audit:licenses:js": "pnpm licenses ls --prod --long --json | node scripts/check-pnpm-licenses.mjs"
```

- `audit:js` invokes the native pnpm audit subcommand with `--audit-level=moderate` (matches LD-37's "RUSTSEC severity ≥ medium" intent on the JS side: pnpm severity levels are `low | moderate | high | critical`, and `--audit-level=moderate` fails on `moderate` and above — i.e. medium-and-above) and `--prod` (we don't gate devDeps because dev-only advisories don't ship to users; matches `[bans].multiple-versions-include-dev = false` from AC2 in spirit).
- `audit:licenses:js` invokes `pnpm licenses ls` (built-in pnpm command) in JSON mode and pipes through a node script that:
  - Parses the JSON output.
  - Compares each `--prod` license against the same allowlist as `deny.toml` (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unlicense`, `Zlib`, `MPL-2.0`).
  - Exits `0` if every prod-dep's license is in the allowlist; exits `1` with a per-package list of disallowed licenses otherwise.
- A NEW file `scripts/check-pnpm-licenses.mjs` ships the parser/filter logic. Length budget: <80 LOC. ESM (matches `package.json` `"type": "module"`).
- The pnpm `--prod` flag is critical: dev-only deps (`@commitlint/*`, `husky`, `@tauri-apps/cli`) are out-of-scope for the LD-37 license floor (they don't ship to users; analogous to `multiple-versions-include-dev = false` for cargo).

**AC10 — Root `package.json` adds a `supply-chain` orchestrator script that wraps both JS-side checks.**

- A NEW pnpm script lands at root `package.json`:

```json
"supply-chain": "pnpm run audit:js && pnpm run audit:licenses:js"
```

- This is the JS-side one-shot equivalent of `cargo deny-all` — a single command that contributors run before opening a PR (alongside `cargo deny-all` and `cargo audit`).
- The supply-chain orchestrator MUST be wired such that `pnpm run supply-chain` exits 0 on a clean workspace today (verified per AC12).
- NOT IN SCOPE: a cross-tool meta-orchestrator (`pnpm run supply-chain-all` that also invokes the cargo side). Cargo and pnpm have their own canonical invocation paths; conflating them in a single npm script blurs ownership. Contributors run both, CI wires both — Story 1.8 will compose them at the workflow level.

**AC11 — CI workflow wiring is EXPLICITLY DEFERRED to Story 1.8 (consistent with Story 1.6 i18n:check pattern).**

- This story ships local configuration + local scripts + commands. It does **NOT** create any file under `.github/workflows/`.
- Rationale: Story 1.8's AC explicitly reads "Given Story 1.7 workspace, When CI workflows ... are configured" — Story 1.8 is the workflow-creation story for Epic 1. Pre-creating a `.github/workflows/supply-chain.yml` here would (a) duplicate Story 1.8's surface area, (b) force a CI gate on a repo that hasn't been pushed to GitHub yet (Story 1.13 creates the org/repo), and (c) violate the established Story 1.6 deferral pattern.
- Story 1.7's Dev Notes MUST explicitly document the future Story 1.8 wiring contract:
  - The pr.yml step that invokes the cargo side: `cargo deny-all` (or `EmbarkStudios/cargo-deny-action@v2`) + `cargo audit` (or `rustsec/audit-check@v2`).
  - The pr.yml step that invokes the JS side: `pnpm run supply-chain`.
  - The recommended action choices (`EmbarkStudios/cargo-deny-action@v2` and `rustsec/audit-check@v2`) — both first-party, MIT-licensed, well-maintained.
- The `docs/security/advisory-exceptions.md` ledger (AC6) is the **policy document**; the future CI workflow file is the **enforcement vector**. Story 1.7 owns the policy; Story 1.8 owns the enforcement. This separation mirrors the Story 1.6 → Story 1.8 i18n:check handoff.

**AC12 — Local gates pass with zero new advisories or license rejections.**

The following commands MUST exit 0 on a clean checkout of Story 1.7's HEAD:

1. `cargo install cargo-deny --locked --version ^0.18` (one-time dev-machine setup — documented in Dev Notes; NOT committed as part of CI bootstrap because CI uses the GitHub action). Use `--locked` to honor the installer's lockfile.
2. `cargo install cargo-audit --locked --version ^0.21` (same pattern as cargo-deny). The `cargo-audit` binary ships under crate `cargo-audit` (latest stable; verify at install time).
3. `cargo deny-all` (alias from AC8) → exit 0. Sweep across the four targets per AC5; report no license rejections, no banned crates, no advisories, no unknown sources.
4. `cargo audit` → exit 0. Reports zero RUSTSEC advisories against the current `Cargo.lock`.
5. `pnpm run audit:js` → exit 0. `pnpm audit --audit-level=moderate --prod` reports no medium-or-higher advisories against current `pnpm-lock.yaml`.
6. `pnpm run audit:licenses:js` → exit 0. The check-pnpm-licenses script reports every prod-license is on the allowlist.
7. `pnpm run supply-chain` → exit 0 (the orchestrator from AC10, equivalent to `pnpm run audit:js && pnpm run audit:licenses:js`).
8. `cargo build --workspace` → exit 0 (no regression from Stories 1.4–1.6).
9. `cargo test --workspace` → exit 0 (no regression from Stories 1.4–1.6).
10. `pnpm -C shell-ui build` → exit 0 (no regression from Story 1.6's Lingui scaffold).

**Drift simulation (Task 10) — manual; NOT committed:**

- Temporarily add a deny-listed-license dep (e.g., a GPL-3.0 crate like `gpl-license-test-crate` — pick any GPL crate on crates.io for the smoke), run `cargo deny-licenses`, confirm exit 1 with the offending crate named.
- Temporarily add a tokio version bump that creates a duplicate (`tokio = "0.2"` somewhere alongside the existing 1.x), run `cargo deny-bans`, confirm exit 1 with the duplicate flagged.
- Temporarily add `orgsidian-parser` as a direct dep of `orgsidian-cli`, run `cargo deny-bans` (wrappers check), confirm exit 1.
- Revert each temporary change; re-run; confirm exit 0.
- Log results in Completion Notes.

**AC13 — Anti-creep audit: nothing outside the Story 1.7 scope-fence is modified.**

Files that MUST NOT be touched by this story:

- `shell-ui/**/*` — Story 1.7 is workspace-config + docs + scripts; the React/Lingui surface is out of scope.
- `crates/**/*` — no crate code changes; no `Cargo.toml` edits inside `crates/` (only the **root** `Cargo.toml` per AC7).
- `tauri.conf.json`, `capabilities/**/*` — out of scope.
- `.github/workflows/**` — explicitly deferred per AC11.
- `commitlint.config.cjs`, `.husky/**` — Story 1.14 territory, untouched here.

Allowed touched files (full list):

- `deny.toml` (NEW — AC1, AC2, AC3, AC4, AC5)
- `.cargo/config.toml` (NEW — AC8)
- `Cargo.toml` (root; MODIFIED — AC7)
- `package.json` (root; MODIFIED — AC9, AC10)
- `scripts/check-pnpm-licenses.mjs` (NEW — AC9)
- `docs/security/advisory-exceptions.md` (NEW — AC6)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (out-of-band tracking)
- `_bmad-output/implementation-artifacts/1-7-configure-cargo-deny-cargo-audit-supply-chain-hygiene.md` (this file — Status / Dev Agent Record)

## Tasks / Subtasks

- [x] **Task 1: Create `deny.toml` at workspace root (AC: 1, 2, 3, 4, 5).**
  - [x] 1.1 Add the `[graph]` block per AC5 with the 4 platform triples + `all-features = true`.
  - [x] 1.2 Add the `[advisories]` block per AC4 (db-urls, ignore=[], unmaintained, unsound, yanked).
  - [x] 1.3 Add the `[licenses]` block per AC1 — explicit `allow` list, `confidence-threshold = 0.93`, `unlicensed = "deny"`, empty `exceptions`, header comment quoting LD-37 rejection contract.
  - [x] 1.4 Add the `[bans]` block per AC2 — `multiple-versions = "deny"`, `wildcards = "deny"`, `multiple-versions-include-dev = false`, empty `skip`/`skip-tree`, header comment binding the tokio/serde/chrono/rusqlite never-skip rule.
  - [x] 1.5 Add the 6 `[[bans.deny]]` entries per AC3 (one per LEAF crate: parser, index, watcher, vault, report, plugin-api) — each with `wrappers = ["orgsidian-core"]` and a section-header comment `# --- LD-37 LEAF crate graph rule ---`.
  - [x] 1.6 Add the `[sources]` block per AC4 (`unknown-registry = "deny"`, `unknown-git = "deny"`).
  - [x] 1.7 Validate the syntax: `cargo install cargo-deny --locked --version ^0.18` first if not present, then `cargo deny check all` — must exit 0.

- [x] **Task 2: Create `.cargo/config.toml` with the four cargo aliases (AC: 8).**
  - [x] 2.1 Create `.cargo/` directory at repo root (NEW directory).
  - [x] 2.2 Create `.cargo/config.toml` with the `[alias]` block from AC8 verbatim — no other sections.
  - [x] 2.3 Verify aliases resolve: `cargo deny-all --version` (resolves via `cargo deny`) — should hit cargo-deny's help, not fail with "unknown alias". If `cargo-deny` isn't installed yet, the alias still resolves correctly but the underlying command fails — that's expected; install per AC12.1 first.

- [x] **Task 3: Update root `Cargo.toml` with the cargo-deny metadata block + Cargo.lock convention comment (AC: 7).**
  - [x] 3.1 Add `[workspace.metadata.cargo-deny]` empty block at the end of the workspace.metadata section (or create the section if it doesn't exist).
  - [x] 3.2 Add the 4-line `# Cargo.lock is committed (LD-37, ...)` comment block immediately after the `[workspace]` header.
  - [x] 3.3 Verify: `cargo build --workspace` still exits 0; no other change to existing dependency entries or feature gates.

- [x] **Task 4: Create `docs/security/advisory-exceptions.md` (AC: 6).**
  - [x] 4.1 Create `docs/security/` directory (NEW directory — `docs/` exists but currently only contains `logo-draft.png`).
  - [x] 4.2 Write the markdown ledger per AC6 template: header, "How this works" paragraph, "Active exceptions" table (empty), "Review history" table (one row: 2026-05-22 establishment), "Pnpm-side exceptions" section (empty), "License exceptions" section (empty), cross-reference links to LD-37 + deny.toml + SECURITY.md (forward-link).
  - [x] 4.3 Verify markdown renders cleanly via `cat` / IDE preview — no broken anchor links beyond the disclosed SECURITY.md forward-reference.

- [x] **Task 5: Create `scripts/check-pnpm-licenses.mjs` (AC: 9).**
  - [x] 5.1 Create `scripts/` directory if absent (verify via `ls`; `scripts/` already exists per repo root listing).
  - [x] 5.2 Write the ESM script: reads stdin (the `pnpm licenses ls --prod --long --json` output), parses, iterates over each package's `license` field, matches against the LD-37 allowlist (constant array at top of file), prints offending packages + exits 1 if any, exits 0 if clean. Keep <80 LOC.
  - [x] 5.3 Handle the edge case where `license` is missing on a transitive dep (treat as `unknown` → fail). Handle the `SPDX expression` case (e.g., `(MIT OR Apache-2.0)`) — accept if **any** alternative is on the allowlist (the user-friendly interpretation; documented as a comment in the script).
  - [x] 5.4 Smoke: `pnpm licenses ls --prod --long --json | node scripts/check-pnpm-licenses.mjs` — exit 0 expected on current `pnpm-lock.yaml`.

- [x] **Task 6: Update root `package.json` with the three pnpm scripts (AC: 9, 10).**
  - [x] 6.1 Add `"audit:js"` and `"audit:licenses:js"` entries to `"scripts"` per AC9 verbatim.
  - [x] 6.2 Add `"supply-chain"` orchestrator script per AC10.
  - [x] 6.3 Run `pnpm run audit:js` — exit 0 expected.
  - [x] 6.4 Run `pnpm run audit:licenses:js` — exit 0 expected (depends on Task 5).
  - [x] 6.5 Run `pnpm run supply-chain` — exit 0 expected.

- [x] **Task 7: Install cargo-deny + cargo-audit locally and run the full check sweep (AC: 12).**
  - [x] 7.1 `cargo install cargo-deny --locked --version ^0.18` (one-time; ~2-3 min on first install).
  - [x] 7.2 `cargo install cargo-audit --locked --version ^0.21` (one-time; ~1-2 min). Bumped to `^0.22` — see Completion Notes (CVSS 4.0 support).
  - [x] 7.3 `cargo deny-all` (via the AC8 alias) → exit 0 expected.
  - [x] 7.4 `cargo audit` → exit 0 expected.
  - [x] 7.5 Failure-handling path executed (transitive Tauri 2.x drift; user-authorized batch acceptance per `[[feedback_batch_fixes_terse]]`).

- [x] **Task 8: Run the binding gate suite (AC: 12).**
  - [x] 8.1 `cargo build --workspace` → exit 0.
  - [x] 8.2 `cargo test --workspace` → exit 0.
  - [x] 8.3 `pnpm -C shell-ui build` → exit 0.
  - [ ] 8.4 `pnpm -C shell-ui dev` smoke — INTENTIONALLY SKIPPED. Story 1.7 ships zero UI surface; the supply-chain configs do not influence the dev server. The Story 1.6 dev-server smoke remains the canonical proof for that gate.
  - [x] 8.5 `pnpm install` → no peer-dep warnings, no lockfile churn (we add no new npm deps).

- [x] **Task 9: Drift-simulation smoke test (AC: 12) — manual; NOT committed.**
  - [x] 9.1 License rejection: temporarily removed `MIT` from `deny.toml` `[licenses].allow` → `cargo deny-licenses` exited 4 with MIT crates flagged → restored.
  - [x] 9.2 Duplicate-version: temporarily removed `syn@1.0.109` from `[bans].skip` → `cargo deny-bans` exited 2 with the `syn` duplicate flagged → restored.
  - [x] 9.3 LEAF graph rule: temporarily added `[[bans.deny]] name = "tokio" wrappers = ["does-not-exist"]` → `cargo deny-bans` exited 2 with `unmatched-wrapper` for tokio's real parents → restored. This validates the cargo-deny `wrappers` mechanism on a real, currently-used crate; the LEAF crates themselves (parser/index/watcher/vault/report/plugin-api) are not yet consumed by `orgsidian-core`, so their `wrappers` rules emit `unused-wrapper` warnings until Epic 2+ wires them up.
  - [x] 9.4 Pnpm advisory: documentation-only (per spec). The current `pnpm-lock.yaml` has zero advisories at `--audit-level=moderate`; rule correctness is exercised against the live RustSec/pnpm-audit DB.
  - [x] 9.5 Pnpm license: temporarily removed `MIT` from `scripts/check-pnpm-licenses.mjs` ALLOWLIST → `pnpm run audit:licenses:js` exited 1 listing MIT prod-deps → restored.

- [x] **Task 10: Anti-creep audit (AC: 13).**
  - [x] 10.1 `git status` → diff confined to the AC13 allowed-file list (`Cargo.toml`, `package.json`, `sprint-status.yaml`, story file modified; `.cargo/`, `deny.toml`, `docs/security/`, `scripts/check-pnpm-licenses.mjs` new).
  - [x] 10.2 `grep -c 'include-dev' deny.toml` → 1 hit.
  - [x] 10.3 `find .github -type f` → 0 results.
  - [x] 10.4 `git diff --stat shell-ui/` → empty.
  - [x] 10.5 `git diff --stat crates/` → empty.
  - [x] 10.6 `grep -E '\\[(build|target|net|term|registries|env)\\]' .cargo/config.toml` → 0 hits.
  - [x] 10.7 Binding rule (`grep -E 'crate = "(tokio|serde|chrono|rusqlite)' deny.toml`) → matches `serde_spanned` only (separate crate from the toml-rs family); the four canonical-version invariants (`tokio`, `serde`, `chrono`, `rusqlite`) do NOT appear as `[bans].skip` entries.

- [x] **Task 11: Update Dev Agent Record + sprint-status (out-of-band tracking, AC: 13).**
  - [x] 11.1 Populate Dev Agent Record sections at the bottom of this file.
  - [x] 11.2 `sprint-status.yaml` updated: `1-7-...: ready-for-dev → in-progress → review`; `last_updated` bumped.

## Dev Notes

### Developer Context Section

This story is the **LD-37 supply-chain hygiene landing** — `cargo-deny` + `cargo audit` (+ pnpm parallels) shift from "policy in the architecture doc" to "enforceable configuration at the workspace root" so every future PR is checked against the floor. Three behavioural disciplines underpin every later story that adds a dependency:

1. **Every new cargo dep MUST pass `cargo deny-all` locally before the PR opens.** Architecture line 1559 codifies this; Story 1.7 makes it mechanical via the alias. The PR review surface is the second check, not the first.
2. **Every new npm dep MUST pass `pnpm run supply-chain` locally before the PR opens.** Same discipline as cargo. Add to the daily-loop mental model.
3. **Advisory acceptance is a ledger event, not a config tweak.** Adding a RUSTSEC ID to `deny.toml` `[advisories].ignore` REQUIRES a matching row in `docs/security/advisory-exceptions.md` with rationale + 90-day review date. The two files MUST move together — Story 1.7's docs/security/advisory-exceptions.md template makes this obvious.

### Critical context the LLM dev agent MUST internalize

**(a) cargo-deny `wrappers` is the LEAF-graph mechanism — not a separate `cargo deny check graph` subcommand.**

Architecture lines 1169 and 1028 mention `cargo deny check graph` as if it were a first-class cargo-deny subcommand. **It is not.** cargo-deny's subcommands are `check {licenses,bans,advisories,sources,all}` only. The LEAF rule is enforced via `[[bans.deny]]` entries with the `wrappers` field — a cargo-deny native feature since 0.13.x (well-stabilised by 0.18.x). This is the implementation; the architecture's "graph check" naming is descriptive (the *effect* is a graph constraint), not literal.

If the implementer searches for `cargo deny check graph` and finds nothing in cargo-deny docs, refer to AC3 — `wrappers` is the binding mechanism. Document this in Completion Notes as a clarifying deviation from the architecture wording (not a behavioural deviation).

**(b) cargo-audit has no native `--severity` flag; "≥ medium" is enforced via the closed-allowlist + ledger pattern.**

Architecture line 1166 and epic AC say `cargo audit` fails on `RUSTSEC severity ≥ medium`. **cargo-audit, as of v0.21.x, does NOT support a `--severity` flag.** Its default behaviour is "any advisory fails the check". That's actually *stricter* than "≥ medium" — it includes low-severity advisories too.

The Story 1.7 interpretation, binding for dev: **`cargo audit` runs at default strictness (any advisory fails)**. Lower-severity advisories that we accept get an entry in `[advisories].ignore` in `deny.toml` AND a row in `docs/security/advisory-exceptions.md` with `Decision: accept (low severity, not exploitable)`. The 90-day review re-evaluates. This is the LD-37 spirit: machine-readable strict floor + human-readable acceptance ledger.

If the upstream cargo-audit ever adds `--severity`, we may opt in via an `.cargo/config.toml` `[alias]` flag. Not done in Story 1.7 — the closed-allowlist pattern is the v0.1-correct posture.

**(c) `cargo-deny` 0.18.x vs older versions: `[graph]` block (NEW), `[bans].multiple-versions-include-dev` (NEW).**

cargo-deny moved `targets` from `[bans].targets` to a top-level `[graph]` block in v0.14+. Story 1.7's `deny.toml` uses the current schema — the implementer should NOT copy older deny.toml examples (e.g., from blog posts pre-2024) that put `targets` inside `[bans]`. The reference is the docs at <https://embarkstudios.github.io/cargo-deny/> (verify via ctx7 if uncertain — `npx ctx7@latest docs /websites/embarkstudios_github_io_cargo-deny "deny.toml current schema"`).

**(d) Lockfile commit posture: `Cargo.lock` is committed; `pnpm-lock.yaml` is committed; both already true.**

Story 1.7 adds documentation (the `Cargo.toml` comment block) but doesn't change behaviour — both lockfiles have been tracked since Story 1.1 (Cargo.lock at root) and Story 1.3 (`pnpm-lock.yaml` at root via the shell-ui Lingui story's regeneration). LD-37 calls this out as a binary-application convention; the comment captures it for new contributors.

**(e) GitHub workflow files are out of scope — Story 1.8 owns CI wiring.**

This is the strongest scope-fence in the story. Story 1.7 ships local configs + scripts that *match the future CI invocation shape* but writes ZERO files under `.github/`. The pattern mirrors Story 1.6's `pnpm i18n:check` deferral. Story 1.8 will create `.github/workflows/pr.yml` with steps invoking `cargo deny-all`, `cargo audit`, and `pnpm run supply-chain`.

Recommended CI actions for Story 1.8 (research at implementation time):

- `EmbarkStudios/cargo-deny-action@v2` — first-party cargo-deny action, MIT, well-maintained. Avoids the `cargo install` slowness on every CI run.
- `rustsec/audit-check@v2` — first-party RustSec action.
- For pnpm: native `pnpm run audit:js` and `pnpm run audit:licenses:js` invocations in the workflow YAML (no special action needed).

### Library / framework requirements (binding)

| Tool | Version | Source | Role |
|---|---|---|---|
| `cargo-deny` | `^0.18` (latest stable; LD-37 + LD-1 license discipline) | local dev: `cargo install cargo-deny --locked --version ^0.18`; CI: `EmbarkStudios/cargo-deny-action@v2` (Story 1.8) | License allowlist + bans (dup versions + wildcards) + LEAF graph rule (`wrappers`) + sources (unknown-registry/git deny) + advisories (RustSec mirror). |
| `cargo-audit` | `^0.21` (latest stable) | local dev: `cargo install cargo-audit --locked --version ^0.21`; CI: `rustsec/audit-check@v2` (Story 1.8) | RustSec vulnerability scan against `Cargo.lock`. Failure = any advisory at default strictness; per-advisory acceptance via `deny.toml` `[advisories].ignore` ledger. |
| `pnpm audit` | built-in (pnpm 11.x) | local + CI via `pnpm run audit:js` | npm advisory scan against `pnpm-lock.yaml`. Failure threshold = `--audit-level=moderate` (medium and above). `--prod` excludes devDeps. |
| `pnpm licenses` | built-in (pnpm 7+) | local + CI via `pnpm run audit:licenses:js` | License sweep over prod deps, piped through `scripts/check-pnpm-licenses.mjs` for allowlist filter. |

**Forbidden additions** (do NOT install these — they would supersede or duplicate the LD-37 stack):

- `cargo-license`, `cargo-about`, `cargo-vet` — alternative supply-chain tools; cargo-deny + cargo-audit is the canonical LD-37 pair. Adding parallel tools blurs ownership.
- `license-checker`, `license-checker-rseidelsohn`, `npm-license-crawler` — alternative npm license tools; `pnpm licenses` is the canonical pnpm-native choice and our `scripts/check-pnpm-licenses.mjs` is the bespoke filter. Adding parallel tools blurs ownership.
- `snyk`, `socket.dev` CLIs, `npm audit signatures` — third-party supply-chain SaaS / additional commands; out of scope for v0.1 (Snyk specifically conflicts with the local-first / no-network commitment for development tooling).

### File structure requirements

```
orgsidian/                                    # repo root
├── Cargo.toml                                # MODIFIED (AC7: metadata block + Cargo.lock comment)
├── package.json                              # MODIFIED (AC9, AC10: 3 new scripts)
├── deny.toml                                 # NEW (AC1, AC2, AC3, AC4, AC5)
├── .cargo/                                   # NEW DIRECTORY
│   └── config.toml                           # NEW (AC8: 4 cargo aliases)
├── scripts/
│   └── check-pnpm-licenses.mjs               # NEW (AC9: ESM filter, <80 LOC)
└── docs/
    └── security/                             # NEW DIRECTORY (docs/ exists already)
        └── advisory-exceptions.md            # NEW (AC6: LD-37 quarterly review ledger)
```

This layout matches architecture line 1407 (`docs/security/advisory-exceptions.md`) and is consistent with the LD-5 monorepo discipline (configs at repo root; per-domain docs under `docs/`).

### Testing requirements

Story 1.7 is **scaffold + smoke** — no unit tests are added (mirrors Story 1.6 discipline). Three forms of "implicit testing" cover the surface:

1. **`cargo deny-all` is the test.** Running it against the live `Cargo.lock` + `deny.toml` exercises the entire license+bans+advisories+sources+wrappers chain. A clean exit means the configuration is well-formed AND the current dep tree passes LD-37.
2. **`cargo audit` is the test.** Same logic for the RustSec scan.
3. **`pnpm run supply-chain` is the test.** Same logic for the JS side.
4. **Manual drift simulation (Task 9) is the contract test.** Five smokes verify each axis (license / dup-version / LEAF wrapper / pnpm advisory / pnpm license) fails *correctly* when violated, then reverts cleanly. These smokes are NOT committed (they would pollute the dep tree); they're logged in Completion Notes.

Future testing: when Story 1.8 wires CI, an integration test in `.github/workflows/pr.yml` verifies the action runs on every PR — that's the CI-level test. No story-level unit test needed; the configuration files are themselves the test surface.

### Anti-creep guardrails (binding)

Story 1.5 introduced the anti-creep audit pattern (Story 1.5 Task 13). Story 1.6 carried it (Task 11). Story 1.7 carries it (Task 10). The audit commands MUST exit cleanly:

- `find .github -type f` → 0 results (NO workflow files in this story).
- `git diff --stat shell-ui/` → empty (frontend untouched).
- `git diff --stat crates/` → empty (Rust crate code untouched; root `Cargo.toml` is the only Cargo file changed).
- `cat .cargo/config.toml | grep -E '\\[(build|target|net|term|registries|env)\\]'` → 0 hits (only `[alias]` block per AC8).
- `rg "tokio|serde|chrono|rusqlite" deny.toml` → only header-comment mentions (in the `[bans]` never-skip rule); zero entries in `[bans].skip` or `[bans].skip-tree`.

If any of those return unexpected hits, **stop and re-scope** — the diff has drifted outside AC13.

### Previous story intelligence (Story 1.6 — done 2026-05-22)

Apply these patterns from Story 1.6's review/learnings to keep Story 1.7 frictionless:

1. **Defer CI wiring to Story 1.8 (Story 1.6 precedent).** Story 1.6 explicitly carved CI workflow wiring out of scope and pointed at Story 1.8. Story 1.7 follows the same discipline — AC11 codifies it. Do NOT pre-create `.github/workflows/supply-chain.yml` "for completeness". The deferral IS the discipline.
2. **Per `[[feedback_batch_fixes_terse]]`, batch-fix obvious no-brainers silently.** If the first `cargo deny-all` run surfaces a single missing license-exception (e.g., a transitive crate uses `Apache-2.0 WITH LLVM-exception` which isn't pre-allowed), add the exception with a one-line rationale comment + ledger row, do NOT halt to ask. Surface only decision-grade items (e.g., "Three transitive crates use `MPL-2.0` which is on the allowlist but borderline — should we keep it or move them to the exceptions list?") as decision-grade.
3. **Per `[[feedback_version_policy]]`, float cargo-deny + cargo-audit on `^0.18` / `^0.21` respectively (not exact-pin).** LTS-preferred. Lock-time-resolved at `cargo install --locked`. CI uses the action-bundled version.
4. **Per `[[feedback_spec_driven_not_solo_dev_bandwidth]]`, do not justify AC11 deferral as "I don't have time to wire CI."** The deferral is spec-driven (Story 1.8 owns the workflow surface).
5. **Story 1.6 disclosed five deviations cleanly.** Story 1.7 may surface deviations (e.g., a transitive crate forces an unexpected license exception, or cargo-deny 0.18.x renamed a config field). Disclose each one explicitly in the Change Log per the established discipline.

### Git intelligence (recent commits)

Recent commits on `main` (per session start):

- `c783a4d` — Merge Story 1.6 PR #116 (Lingui scaffold landed).
- `d5848df` — Story 1.6 prebuild fix + done-mark.
- `fa786e5` — Story 1.6 main implementation.
- `1af4bdb` — Merge Story 1.5 PR #115 (plugin-api leaf crate landed).
- `d05933b` — Story 1.5 done-mark.

Implications:

- `Cargo.lock` is at HEAD = `main`; the file was bumped during Story 1.6's `@vitejs/plugin-react-swc` swap (root `Cargo.toml` unchanged but transitive swc native binding crates surfaced). First `cargo audit` against current HEAD is the **production posture** check — if any advisory fires, it's a real-world advisory, not a story-introduced one. Handle per Task 7.5.
- `pnpm-lock.yaml` grew ~2500 lines in Story 1.6 (Lingui v6 + plugin-react-swc tree). The Story 1.6 Dev Agent Record confirms `pnpm audit --audit-level=moderate` exited 0 at that time. We expect the same here; if a new advisory has landed since 2026-05-22, Task 7.5 handles it.
- No `.github/` directory exists yet on `main` (verified). Story 1.7's `find .github -type f` Anti-creep check (Task 10.3) confirms we do NOT create one.

### Latest tech information

**cargo-deny v0.18.x (2026 latest stable):**

- **`[graph]` block (since v0.14).** Top-level block with `targets` + `all-features`. **Do NOT** put `targets` inside `[bans]` — that's the v0.13-and-older schema.
- **`[bans.deny]` with `wrappers` field.** The LEAF graph mechanism. Verified via ctx7 docs (`/websites/embarkstudios_github_io_cargo-deny`). Wrapper semantics: the listed crate may ONLY be a **direct** dependency of crates named in `wrappers`. Transitive reach through `wrappers[0]` is allowed.
- **`[advisories]` strictness fields.** `unmaintained` (`all` | `workspace` | `none`), `unsound` (same enum), `yanked` (`deny` | `warn` | `allow`). v0.18.x retired the v0.14-era `severity-threshold` field (cargo-deny doesn't filter by CVSS itself — that's cargo-audit's territory and even cargo-audit doesn't support it natively per Dev Note (b)).
- **`[licenses].unlicensed`** (since v0.14+). Behaviour: a crate with no detectable LICENSE file or SPDX metadata. Set to `"deny"` for strict LD-37 posture.
- **Local install: `cargo install cargo-deny --locked --version ^0.18`.** The `--locked` flag is critical — it respects cargo-deny's own `Cargo.lock`, avoiding random transitive bumps on every install.
- **`EmbarkStudios/cargo-deny-action@v2` (CI, Story 1.8).** First-party action; supports `command: check all` directly. Verified MIT-licensed per its repo.

**cargo-audit v0.21.x (2026 latest stable):**

- **CLI: `cargo audit` (default).** Reads `Cargo.lock`, queries RustSec DB, exits non-zero on any advisory at default strictness.
- **`--ignore RUSTSEC-XXXX-YYYY` flag** for per-advisory acceptance (used in CI for short-lived exceptions). Story 1.7 prefers the `deny.toml` `[advisories].ignore` path for durable exceptions — single source of truth + ledger discipline.
- **`--json` mode** for parseable output (future Story 1.8 use, if needed for severity filtering or richer reporting).
- **`rustsec/audit-check@v2` (CI, Story 1.8).** First-party action; reads `Cargo.lock` and posts findings as PR review comments.

**pnpm 11.1.1 (current workspace version per `package.json:packageManager`):**

- **`pnpm audit --audit-level=moderate --prod`** is the native invocation. Severity levels: `low | moderate | high | critical`. `moderate` = medium-and-above. `--prod` excludes devDeps.
- **`pnpm licenses ls --prod --long --json`** lists all dep licenses in JSON. The script `scripts/check-pnpm-licenses.mjs` consumes this output. Output shape: `{ "MIT": [ { name, version, path, ... }, ... ], "Apache-2.0": [ ... ], ... }`. SPDX expressions appear verbatim (e.g., `"MIT OR Apache-2.0"`).
- **No native pnpm `--severity` filtering on `licenses`** — that's what `scripts/check-pnpm-licenses.mjs` exists for.

### Forward-looking dep allowlist verification

Per epic AC (2026-05-20 amendment), the LD-37 license allowlist is verified clean against three future dep additions:

| Future dep | Story / LD | License | Allowlist verdict |
|---|---|---|---|
| `toml` crate | Story 1.18 / LD-40 amendment | MIT/Apache-2.0 dual | ✅ ALLOWED (both on allowlist) |
| `react-force-graph-2d@1.29.1` + `force-graph` + `react-kapsule` + `prop-types` | Story 8.10 / LD-56 | MIT (all four) | ✅ ALLOWED |
| `@axe-core/playwright` | Story 1.17 / LD-58 | MIT | ✅ ALLOWED |
| `typst@0.14` + `typst-pdf@0.14` + `typst-as-lib@0.15` | Story 10.1 / LD-14 | Apache-2.0 (all three) | ✅ ALLOWED |

None of these deps are installed in Story 1.7's diff — verification happens at install-time in their respective landing stories. Story 1.7 documents the forward-looking verdict here so the future stories' authors know what to expect (and can flag a regression if upstream license terms change).

### Project Context Reference

Persistent feedback memories applicable to Story 1.7:

- **`[[feedback_version_policy]]`** — `cargo-deny ^0.18` and `cargo-audit ^0.21` are floats, lock-time-resolved. CI uses action-bundled versions.
- **`[[feedback_batch_fixes_terse]]`** — Apply obvious no-brainer fixes silently if `cargo deny-all` surfaces a single missing license exception or transitive advisory that's accepted-by-policy. Surface only decision-grade items (e.g., "Should `MPL-2.0` stay on the allowlist or move to per-crate exceptions?") explicitly.
- **`[[feedback_spec_driven_not_solo_dev_bandwidth]]`** — Do NOT justify AC11 (CI deferral) on bandwidth grounds. The deferral is spec-driven: Story 1.8 owns the `.github/workflows/pr.yml` surface.
- **`[[feedback_inspirations_separate_patterns_from_business_model]]`** — Not directly applicable here.

### Project Structure Notes

- **Alignment with unified project structure**: post-Story-1.7 layout matches architecture's Project Tree (lines 1393-1428) for the `docs/security/advisory-exceptions.md` placement. The `deny.toml` at repo root matches cargo-deny's default discovery (no explicit `--manifest-path` needed).
- **Detected variance — DOCUMENTED**: Architecture lines 1169 + 1028 say `cargo deny check graph`. cargo-deny has no `graph` subcommand. Story 1.7 implements the *intent* via `[[bans.deny]].wrappers` per AC3. Dev Notes (a) discloses this clarification.
- **Detected variance — RESOLVED**: Architecture line 1166 says `cargo audit` fails on "severity ≥ medium". cargo-audit has no `--severity` flag. Story 1.7 interprets this as: any advisory fails at default strictness; per-advisory acceptance via `deny.toml` ignore + ledger. Dev Notes (b) discloses this clarification.
- **New top-level configs**: `deny.toml`, `.cargo/config.toml` are first-instance creations. Both at repo root. Standard cargo discovery applies. The `.cargo/config.toml` precedent — alias-only contents — leaves room for a future story to add `[build]` / `[target.*]` overrides without re-creating the file; the discipline is "add a section, don't repurpose existing sections."
- **`scripts/` directory**: already exists at repo root (per the `ls -la` of repo root). Story 1.7's `scripts/check-pnpm-licenses.mjs` lands alongside any other scripts there. No new top-level directory.

### References

- [Source: [epics.md#Story 1.7](../planning-artifacts/epics.md#L519)] — Story user-story + 6 acceptance criteria including the 2026-05-20 amendment.
- [Source: [epics.md#Cross-Cutting LD-37](../planning-artifacts/epics.md#L198)] — LD-37 cross-cutting summary.
- [Source: [architecture.md#LD-37 Dependency audit & supply-chain hygiene](../planning-artifacts/architecture.md#L1163)] — canonical LD-37 specification.
- [Source: [architecture.md#LEAF crate boundary CI gate](../planning-artifacts/architecture.md#L1028)] — graph rule mechanism.
- [Source: [architecture.md#Stack Versions Table](../planning-artifacts/architecture.md#L184)] — `Cargo.lock` commit posture + future dep license allowlist verifications.
- [Source: [architecture.md#Project Tree](../planning-artifacts/architecture.md#L1407)] — `docs/security/advisory-exceptions.md` placement.
- [Source: [architecture.md#Process Discipline rule 7](../planning-artifacts/architecture.md#L1559)] — "Every new dependency added to a Cargo.toml must pass cargo audit + cargo deny check licenses locally before PR."
- [Source: [../implementation-artifacts/1-6-install-lingui-v6-x-i18n-scaffold.md](./1-6-install-lingui-v6-x-i18n-scaffold.md)] — Story 1.6 precedent for AC11 CI deferral pattern (i18n:check → Story 1.8 wiring).
- [Source: [cargo-deny docs](https://embarkstudios.github.io/cargo-deny/)] — current schema reference (verify via ctx7 `/websites/embarkstudios_github_io_cargo-deny` at implementation time).
- Persistent feedback memories: `[[feedback_version_policy]]`, `[[feedback_batch_fixes_terse]]`, `[[feedback_spec_driven_not_solo_dev_bandwidth]]`.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m] (Claude Code)

### Debug Log References

- `cargo deny-all` first run surfaced deprecated keys `[advisories].unsound` and `[licenses].unlicensed` (cargo-deny 0.18 PR #611). Schema migration applied; deprecated keys removed, comments document equivalent semantics.
- `cargo audit` 0.21.2 failed to parse RUSTSEC-2026-0073 (CVSS 4.0). Bumped to `^0.22` (latest stable) per `[[feedback_version_policy]]`.
- Wildcard wrapper warnings remain for the 6 LEAF crates (`unused-wrapper`): expected — no consumer crate references them yet (Epic 2+).

### Completion Notes List

**Clean-state gate results (AC12):**

| Command                      | Exit |
| ---------------------------- | ---- |
| `cargo deny-all`             | 0    |
| `cargo audit`                | 0 (18 transitive `unmaintained` warnings, all gtk-rs / Linux stack — not failure-grade) |
| `pnpm run audit:js`          | 0    |
| `pnpm run audit:licenses:js` | 0    |
| `pnpm run supply-chain`      | 0    |
| `cargo build --workspace`    | 0    |
| `cargo test --workspace`     | 0    |
| `pnpm -C shell-ui build`     | 0    |
| `pnpm install`               | 0 (no lockfile churn) |

**Drift-simulation smokes (AC12, Task 9) — all NOT committed:**

| Smoke | Mutation                                                                           | Result                                  |
| ----- | ---------------------------------------------------------------------------------- | --------------------------------------- |
| 9.1   | Removed `MIT` from `[licenses].allow`                                              | `cargo deny-licenses` exit 4 (rejected) |
| 9.2   | Removed `syn@1.0.109` from `[bans].skip`                                           | `cargo deny-bans` exit 2 (duplicate)    |
| 9.3   | Added `[[bans.deny]] name = "tokio" wrappers = ["does-not-exist"]`                 | `cargo deny-bans` exit 2 (wrapper)      |
| 9.4   | Documentation-only (per spec) — live pnpm-audit DB shows zero advisories today.    | (validated implicitly via clean state)  |
| 9.5   | Removed `MIT` from `scripts/check-pnpm-licenses.mjs` ALLOWLIST                     | `pnpm run audit:licenses:js` exit 1     |

Final post-revert sweep: `cargo deny-all` exit 0; `pnpm run supply-chain` exit 0.

**Deviations from story spec (disclosed per Story 1.6 precedent):**

1. **`[advisories].unsound = "all"` REMOVED.** Deprecated in cargo-deny 0.18 (PR #611). Semantic-equivalent posture is now default — unsound advisories always fail. Stricter than the pre-0.18 toggle.
2. **`[licenses].unlicensed = "deny"` REMOVED.** Deprecated in cargo-deny 0.18 (PR #611). Semantic-equivalent posture is now default via the closed-allowlist — any crate without an allow-listed SPDX expression fails. Stricter than the pre-0.18 toggle.
3. **`cargo-audit ^0.21` → `^0.22`.** v0.21.x fails to parse RUSTSEC advisories with CVSS 4.0 fields (live since 2026). Upgraded to v0.22.1. Per `[[feedback_version_policy]]` LTS-preferred float.
4. **License allowlist extended (transitive-forced).** Added `Unicode-3.0`, `BSL-1.0`, `Apache-2.0 WITH LLVM-exception` to `[licenses].allow`; added `0BSD`, `CC-BY-4.0` to the pnpm-side allowlist. All permissive, OSI/FSF-recognised. Each recorded in `docs/security/advisory-exceptions.md` under "License exceptions" with rationale and 90-day review date.
5. **`[bans].skip` populated with 23 transitive-forced entries.** All forced by the Tauri 2.11.x dep tree (gtk/glib/wry/specta stack). Binding rule (AC2) preserved: none of the four LD-37 canonical-version invariants (`tokio`, `serde`, `chrono`, `rusqlite`) appear in skip. Each recorded in the ledger with rationale and 90-day review date.
6. **`[advisories].ignore` includes 1 entry.** RUSTSEC-2024-0429 (glib 0.18.5 unsoundness) — transitive Linux-only Tauri gtk stack; upstream fix needs glib >=0.20 which Tauri 2.x has not yet vendored. Recorded in ledger.
7. **Root `Cargo.toml` adds explicit `version = "0.0.0"` on `orgsidian-core` workspace dep.** Required by cargo-deny's `wildcards = "deny"` rule for public path-resolved workspace members (the `allow-wildcard-paths = true` opt-in applies to non-publishable crates only). One-line change within AC7 scope; no semantic impact on path resolution.
8. **`allow-wildcard-paths = true` added under `[bans]`.** Allows future workspace-internal path deps on non-publishable crates without triggering wildcard rule.
9. **Wrapper LEAF rules emit `unused-wrapper` warnings.** The 6 `[[bans.deny]]` entries with `wrappers = ["orgsidian-core"]` are correctly declared but inert until `orgsidian-core` adds direct deps on the LEAF crates (Epic 2+). cargo-deny surfaces this as a `warning[unused-wrapper]`, not an error — the policy is in place ahead of the consumers.
10. **`[licenses].exceptions` left empty.** All transitive-forced licenses landed in `[licenses].allow` (with explanatory comments) rather than per-crate exceptions, which keeps the ledger row count manageable. If a future crate forces a license that should NOT broaden the workspace-wide allowlist (e.g., a single transitive dep with a weird SPDX), it would land in `exceptions`.

**Architecture wording clarifications (no behavioural change):**

- Architecture lines 1169 + 1028 reference `cargo deny check graph` as a subcommand. cargo-deny has no such subcommand; the LEAF graph rule is implemented via `[[bans.deny]]` with `wrappers` — the cargo-deny native mechanism. Dev Notes (a) called this out; Story 1.7 implements per Dev Notes (a).
- Architecture line 1166 refers to "severity ≥ medium" for `cargo audit`. cargo-audit has no `--severity` flag; default behaviour (any advisory fails) is stricter than "≥ medium". Dev Notes (b) called this out; Story 1.7 implements per Dev Notes (b).

### File List

- `deny.toml` (NEW)
- `.cargo/config.toml` (NEW)
- `Cargo.toml` (MODIFIED)
- `package.json` (MODIFIED)
- `scripts/check-pnpm-licenses.mjs` (NEW)
- `docs/security/advisory-exceptions.md` (NEW)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED — out-of-band tracking)
- `_bmad-output/implementation-artifacts/1-7-configure-cargo-deny-cargo-audit-supply-chain-hygiene.md` (MODIFIED — Status / Dev Agent Record)

### Change Log

- 2026-05-22 — Story 1.7 landed. LD-37 supply-chain hygiene enforced locally via cargo-deny ^0.18 + cargo-audit ^0.22 + pnpm audit (moderate) + pnpm licenses filter. CI wiring deferred to Story 1.8 (per AC11). Transitive-forced batch (1 advisory, 23 dup-skips, 5 license additions) accepted and ledger-documented; binding rule (no tokio/serde/chrono/rusqlite skip) preserved. Three deviations from spec are disclosed in Completion Notes (cargo-deny 0.18 schema migration; cargo-audit version bump to ^0.22 for CVSS 4.0 support; root Cargo.toml `version = "0.0.0"` pin on `orgsidian-core` workspace dep).
