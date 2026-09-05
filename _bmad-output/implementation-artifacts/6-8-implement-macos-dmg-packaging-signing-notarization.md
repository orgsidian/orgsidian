---
title: 'Implement macOS DMG packaging + signing + notarization'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: '781bc36'
review_loop_iteration: 0
github_issue: 59
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 1 through Story 6.7 ship a working app that only ever runs from `cargo tauri dev`/`build` on a developer machine. There is no release pipeline: no workflow builds a distributable macOS artifact, nothing signs it with the Apple Developer ID Application certificate (LD-19), nothing notarizes it, and nothing publishes it anywhere a macOS adopter could find it (LD-34). Without this story, v0.1 Alpha has no installable artifact at all — Story 6.10's "publish v0.1 Alpha" has nothing to point to.

**Approach:** Add `.github/workflows/release.yml`, triggered by pushing a `v0.1.0-alpha.x` tag (the LD-33 `v*` tag scheme, matching `cliff.toml`'s `tag_pattern`). A `macos-dmg` job (runs on `macos-14`, matching pr.yml's pinned runner) imports the Apple Developer ID Application certificate into a throwaway CI keychain, resolves the signing identity, then delegates the actual build+sign+notarize+publish to the official `tauri-apps/tauri-action`, targeting `aarch64-apple-darwin` (macOS-arm64 per the AC). Notarization (via `notarytool`, staple attached) is the Tauri bundler's built-in behavior once `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` are present in the job environment — no separate notarization step is scripted by hand. The action creates a draft, prerelease GitHub Release for the pushed tag with the signed DMG attached. A second job, `homebrew-cask`, downloads that DMG, hashes it, renders `packaging/homebrew/orgsidian.rb.tmpl`, and pushes the rendered cask to the external `orgsidian/tap` repository (LD-34) via `scripts/publish-homebrew-cask.sh`.

Signing/notarization/tap-publish cannot be exercised in this sandbox — no Apple Developer ID certificate, no Apple ID, and no `orgsidian/tap` PAT exist here. The workflow shape and every secret name were verified against the current Tauri 2.x docs (`tauri-apps/tauri-docs`, "Distribute → Sign → macOS" + "Distribute → Pipelines → GitHub", fetched 2026-09-05 via the project's `ctx7` documentation rule) rather than guessed from training-data recall. See `docs/releasing.md` for the full "Required repository secrets" handoff and what remains unverified until a maintainer runs a real tagged release.

## Boundaries & Constraints

**Always:**
- Trigger stays `push: tags: ['v0.1.0-alpha.*']` (LD-33's `v*` scheme) plus `workflow_dispatch` for a manual re-run.
- Every secret referenced by name in the workflow must also be documented (name, purpose, how to obtain it) in `docs/releasing.md`'s "Required repository secrets" table — no undocumented secret reference.
- No real certificate, password, Apple ID, team ID, or PAT is ever hardcoded anywhere in this repo — secrets only via `${{ secrets.* }}`, consumed as job/step `env:`.
- Action versions semver-major-pinned (`@v5`, `@v1`, `@v2`); runner images pinned (`macos-14`, `ubuntu-24.04`) — never `*-latest`, matching `pr.yml`'s existing convention ([[feedback_version_policy]]).
- The GitHub Release this workflow creates is a **draft** + **prerelease** — a human publishes it; this story does not do the "go public" step (that is Story 6.10's job).
- Follow the official `tauri-apps/tauri-action` path for the build/sign/notarize/publish step rather than hand-rolling bundler invocations, per the task's explicit instruction.

**Ask First:**
- Any change to the `v0.1.0-alpha.*` tag pattern or to `cliff.toml`'s `tag_pattern` (they must stay in lockstep).
- Adding a new external crate dependency (none needed for this story; it is CI/release infra only).

**Never:**
- Do not modify `sprint-status.yaml`.
- Do not commit, push, or open a PR — the orchestrator handles git/PR for this worktree.
- Do not attempt to create the external `orgsidian/tap` repository itself — scaffold the publish path only, document the manual bootstrap step.
- Do not implement the Linux AppImage (Story 6.9) or Windows MSI jobs in this file beyond leaving them as documented future additions to the same workflow.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| `v0.1.0-alpha.x` tag pushed, all secrets configured | tag matches trigger pattern | `macos-dmg` builds + signs + notarizes the arm64 DMG, `tauri-action` opens a draft prerelease with the DMG attached | N/A (only verifiable in a real tagged run) |
| `APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD` missing or wrong | cert import fails or resolves no identity | "Resolve signing identity" step fails fast with `::error::` pointing at `docs/releasing.md` | job fails, no partial/unsigned release published |
| Imported cert is not a "Developer ID Application" cert | e.g. an "Apple Development" cert | `security find-identity \| grep "Developer ID Application"` finds nothing → explicit error, job fails | job fails before any build step runs |
| `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` missing | notarization credentials absent | Tauri bundler skips notarization per its own `notarize_auth()` fallback (warns, does not hard-fail signing) — but Story 6.8's AC requires notarization, so these three secrets are treated as required in `docs/releasing.md`, not optional | maintainer-configured; not enforced at the YAML level (Tauri CLI's own behavior) |
| `macos-dmg` succeeds but the release has no `.dmg` asset yet when `homebrew-cask` runs | (should not happen — `needs: macos-dmg` orders the jobs) | `publish-homebrew-cask.sh` errors explicitly ("No .dmg asset found...") rather than pushing a bad cask | job fails, no cask pushed |
| `HOMEBREW_TAP_GITHUB_TOKEN` missing or lacks write access to `orgsidian/tap` | PAT absent/wrong scope | `git push` in the publish script fails; the `: "${VAR:?msg}"` guards fail fast for the missing-entirely case | job fails, no cask pushed |
| Re-run on the same tag after a prior successful cask publish | `Casks/orgsidian.rb` already matches | script diffs the rendered file against the clone and exits 0 with "already up to date" — no empty commit | N/A |
| DMG asset name doesn't match `*.dmg` glob (bundler naming drift) | unexpected filename | `gh release download --pattern '*.dmg'` still matches by extension, not exact name, so this is resilient to product-name/arch-suffix changes | if genuinely zero `.dmg` assets exist, same explicit error as above |

</frozen-after-approval>

## Code Map

- `.github/workflows/release.yml` -- NEW. `macos-dmg` job (checkout w/ submodules, Rust toolchain + cache, pnpm/Node, JS deps, Apple cert import + identity resolution, `tauri-apps/tauri-action@v1` build+sign+notarize+publish, keychain cleanup) + `homebrew-cask` job (`needs: macos-dmg`, invokes the publish script). Header comment carries the full "Required repository secrets" list pointing at `docs/releasing.md`.
- `packaging/homebrew/orgsidian.rb.tmpl` -- NEW. Homebrew cask template with `__VERSION__`/`__URL__`/`__SHA256__` placeholders; `depends_on arch: :arm64` (v0.1.0-alpha.x is arm64-only); standard `zap trash:` block for the app's `com.orgsidian.app` identifier.
- `scripts/publish-homebrew-cask.sh` -- NEW. Looks up the release's `.dmg` asset via `gh release view`/`download`, computes its sha256, renders the template, clones `orgsidian/tap` with `HOMEBREW_TAP_GITHUB_TOKEN`, and pushes `Casks/orgsidian.rb` if it changed. Idempotent (no-op commit on a re-run with no changes). `set -euo pipefail` + `: "${VAR:?...}"` guards for every required env var.
- `docs/releasing.md` -- NEW. The decision-grade "Required repository secrets" table (name, purpose, how to obtain), what the workflow does step by step, the manual bootstrap items this story does NOT automate (creating `orgsidian/tap`, minting the tap PAT, publishing the draft release), and an explicit "what was validated vs. what can only be validated in a real tagged run" section.

No Rust or TypeScript source changed — this story is CI/release infrastructure + docs only. `crates/orgsidian-shell-app/tauri.conf.json` needed no changes: `bundle.targets: "all"` already includes `dmg` on macOS and `icons/icon.icns` already exists, so the DMG target has everything the Tauri bundler needs without a signing identity hardcoded in config (the identity is supplied purely via the `APPLE_SIGNING_IDENTITY` env var at release time, never checked into config).

## Tasks & Acceptance

**Execution:**
- [x] `.github/workflows/release.yml`: tag trigger, `macos-dmg` job (cert import + identity resolution + `tauri-action` build/sign/notarize/publish + keychain cleanup).
- [x] `.github/workflows/release.yml`: `homebrew-cask` job wired to `scripts/publish-homebrew-cask.sh`.
- [x] `packaging/homebrew/orgsidian.rb.tmpl` cask template.
- [x] `scripts/publish-homebrew-cask.sh` (asset lookup, hash, render, clone+push, idempotent).
- [x] `docs/releasing.md` required-secrets handoff + runbook.
- [x] YAML validated (`actionlint`); script validated (`shellcheck`); `cargo build --workspace --offline` reconfirmed unaffected.

**Acceptance Criteria:**
- Given Epic 1 + Stories 6.1-6.6, when a `v0.1.0-alpha.x` tag is pushed, then `.github/workflows/release.yml` builds the macOS-arm64 DMG via the Tauri bundler. *(Trigger + `args: --target aarch64-apple-darwin` + `projectPath: crates/orgsidian-shell-app` wired; only exercisable in a real tagged CI run — see docs/releasing.md.)*
- And the DMG is signed with the Apple Developer ID Application certificate (key stored as a GitHub Actions secret). *(`APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD`/`KEYCHAIN_PASSWORD` import + `APPLE_SIGNING_IDENTITY` resolution wired per the ctx7-verified tauri-docs sequence, grepping specifically for a "Developer ID Application" identity.)*
- And the DMG is notarized via `notarytool` and the staple is attached. *(`APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` passed to `tauri-action`; notarization + stapling is the Tauri bundler's built-in behavior when these are present — ctx7-verified against `crates/tauri-bundler`'s macOS `app.rs` notarize path. Cannot be executed without real Apple credentials in this sandbox.)*
- And the artifact is uploaded to the GitHub Release page. *(`tauri-action`'s `releaseDraft: true`/`prerelease: true` creates the release for the pushed tag with the DMG attached — this is the action's own upload behavior, not scripted separately.)*
- And a Homebrew cask formula is published to `orgsidian/tap` per LD-34. *(`homebrew-cask` job + `scripts/publish-homebrew-cask.sh` + `packaging/homebrew/orgsidian.rb.tmpl`. Cannot push to the real `orgsidian/tap` without `HOMEBREW_TAP_GITHUB_TOKEN` and without that repo existing — both are documented manual maintainer steps in `docs/releasing.md`.)*

## Design Notes

- **Why delegate to `tauri-apps/tauri-action` instead of hand-rolling `pnpm tauri build`.** The task explicitly asks for the official action's approach where it fits, and it fits cleanly here: the action already knows how to invoke the bundler with the right target, wire `APPLE_SIGNING_IDENTITY`/`APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` through to the Tauri CLI's own notarization logic, and create+attach-to a GitHub Release. Reimplementing that by hand would duplicate logic the action already tests upstream, for no benefit.
- **Why the certificate import step is still hand-written (not delegated to the action).** `tauri-action` does not import the certificate into a keychain itself — that half of the ctx7-verified reference workflow is deliberately a separate step before the action runs, matching the current official docs exactly (down to the `security` invocation sequence). Adapted from the doc's own "Apple Development" example to grep "Developer ID Application" instead, since LD-19 specifically calls for the distribution-class certificate, not a development one.
- **Why `tagName` is left unset in the `tauri-action` `with:` block.** The action's `__VERSION__` auto-tagging convention documented for the "push to a release branch" trigger mode doesn't apply here: this workflow triggers on an already-pushed tag, so the action publishes the release under `github.ref_name` directly. Setting `tagName` explicitly would risk fighting the tag that already exists.
- **Why the DMG lookup in the publish script globs `*.dmg` instead of hardcoding a filename.** The exact bundler output filename (`{productName}_{version}_{arch}.dmg`-style conventions) is an implementation detail of the Tauri bundler that this story could not independently re-verify byte-for-byte offline; matching by extension via `gh release download --pattern '*.dmg'` is robust to that detail without weakening the check meaningfully (a release job that produced zero DMGs still fails loudly).
- **Why the cask template lives in this repo rather than being generated inline in the workflow.** Keeping `packaging/homebrew/orgsidian.rb.tmpl` as a real file (vs. a heredoc in YAML) makes cask changes reviewable as a normal diff and keeps `scripts/publish-homebrew-cask.sh` a thin, testable renderer rather than a place where cask content and push logic are tangled together.
- **Honest scope boundary.** This story cannot exercise signing, notarization, or the tap push in this sandbox — there is no Apple Developer ID certificate, no Apple ID, and no `orgsidian/tap` PAT here, and the `orgsidian/tap` repository itself does not exist yet. What's shipped is a workflow whose shape, secret names, and step sequence are verified against current upstream docs (not training-data recall), plus fail-fast error messages at every point a misconfigured secret would otherwise produce a confusing downstream failure. `docs/releasing.md` is the decision-grade handoff spelling out exactly what a maintainer must do before the first real tag push.

## Verification

**Commands:**
- `git submodule update --init --recursive` -- expected: `tree-sitter-org` submodule present (was required before `cargo build` succeeded in this sandbox).
- `actionlint .github/workflows/release.yml` -- expected: no output (clean).
- `shellcheck scripts/publish-homebrew-cask.sh` -- expected: no output (clean).
- `cargo build --workspace --offline` -- expected: builds clean (this story touches no Rust source; confirms nothing else broke).

**Result (2026-09-05):** `actionlint` clean on `release.yml`. `shellcheck` clean on `publish-homebrew-cask.sh` (one `SC2034` unused-variable warning was found and fixed by removing the dead `ASSET_URL` assignment). `cargo build --workspace --offline` finished successfully (18.39s, all 9 workspace crates) after initializing the `tree-sitter-org` submodule — no regressions from this story's changes, which touch only `.github/workflows/`, `packaging/`, `scripts/`, and `docs/`. Signing, notarization, and the `orgsidian/tap` push are **not validated** here — they require a real Apple Developer ID certificate, Apple ID, and tap PAT, none of which exist in this sandbox; they can only be confirmed by a maintainer pushing a real `v0.1.0-alpha.x` tag with the secrets from `docs/releasing.md` configured.

## Spec Change Log

- 2026-09-05 — Implemented. `.github/workflows/release.yml` (`macos-dmg` + `homebrew-cask` jobs), `packaging/homebrew/orgsidian.rb.tmpl`, `scripts/publish-homebrew-cask.sh`, `docs/releasing.md`. Workflow shape + secret names verified against current Tauri 2.x docs via `ctx7` (not guessed). Signing/notarization/tap-publish unverifiable offline — documented as the explicit decision-grade handoff in `docs/releasing.md`. Status → review.
