---
title: 'Implement Linux AppImage packaging'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: '9917329'
review_loop_iteration: 0
github_issue: 60
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 6.8 gives macOS adopters a signed, notarized DMG on GitHub Releases, but `.github/workflows/release.yml` still builds nothing for Linux. A Linux adopter on Ubuntu or Arch has no installable artifact at all — Story 6.10's "publish v0.1 Alpha" would have only a macOS download to point at, undercutting LD-34's cross-platform distribution goal and leaving a real slice of SM-1's target audience (the "Linux adopter" persona in this story's own user-story framing) unaddressed.

**Approach:** Add a `linux-appimage` job to the same `.github/workflows/release.yml` Story 6.8 created, as a sibling to `macos-dmg` rather than a new workflow file (matching this story's explicit "extends the release pipeline" framing). The job runs on `ubuntu-22.04` — a deliberate exception to the repo's usual `ubuntu-24.04` pin, because the Tauri 2.x AppImage docs (ctx7-verified) call for building on the *oldest* base system you intend to support, to avoid raising the minimum glibc version the resulting AppImage requires. It installs the WebKitGTK/AppImage system dependencies (merged from two ctx7-verified `tauri-apps/tauri-docs` pages — the canonical Debian/Ubuntu prerequisites list plus the AppImage-specific extras from the GitHub Actions pipeline reference), imports a dedicated GPG signing key, resolves its key ID, then delegates the build+sign+publish to `tauri-apps/tauri-action` (matching the macOS job's pattern) with appimagetool's built-in GPG signing enabled (`SIGN=1`/`SIGN_KEY`/`APPIMAGETOOL_SIGN_PASSPHRASE`/`APPIMAGETOOL_FORCE_SIGN=1`, per the ctx7-verified "Distribute → Sign → Linux" doc). A follow-up step computes a `SHA256SUMS` file over the built AppImage, GPG-detached-signs it (`SHA256SUMS.asc`), and uploads both to the same release via `gh release upload`.

Critically, `linux-appimage` declares `needs: macos-dmg` — both jobs target the same tag's GitHub Release, and `tauri-apps/tauri-action` looks up (or creates) that release on every invocation. Running them in parallel risks both jobs racing to *create* the release the first time a tag is pushed, with the loser failing outright on GitHub's API rejection rather than gracefully appending. Serializing behind `macos-dmg` guarantees the release always already exists (as a draft) by the time `linux-appimage`'s `tauri-action` step runs, so it deterministically appends rather than races.

A best-effort Flathub manifest scaffold is added under `packaging/flatpak/` per LD-34's "filed best-effort" language — explicitly NOT submitted, and explicitly not submission-ready (it repackages a release AppImage rather than building from source, which Flathub's review process would reject; see the manifest's own header comment and `docs/releasing.md`'s "Flathub (best-effort, not submitted)" section for the concrete gaps).

Nothing in this story can be exercised end-to-end in this sandbox — no Linux GitHub Actions runner, no GPG signing key, and no real tagged release exist here. The workflow shape, system-dependency package names, and the GPG-signing env-var contract were verified against the current Tauri 2.x docs (`tauri-apps/tauri-docs`, "Distribute → Sign → Linux", "Distribute → Pipelines → GitHub", "Start → Prerequisites", "Distribute → AppImage", fetched 2026-09-05 via the project's `ctx7` documentation rule) rather than guessed from training-data recall.

## Boundaries & Constraints

**Always:**
- `linux-appimage` lives in the SAME `.github/workflows/release.yml` file Story 6.8 created — not a new workflow — and uploads to the SAME GitHub Release as `macos-dmg` (the draft, prerelease release for the pushed tag).
- `needs: macos-dmg` on the `linux-appimage` job — no parallel release-creation race.
- Every secret referenced by name in the workflow must also be documented (name, purpose, how to obtain it) in `docs/releasing.md`'s "Required repository secrets" table alongside the Story 6.8 secrets — no undocumented secret reference.
- No real GPG private key or passphrase is ever hardcoded anywhere in this repo — secrets only via `${{ secrets.* }}`, consumed as job/step `env:`.
- Action versions semver-major-pinned (matching Story 6.8's `@v5`/`@v1` convention); the `linux-appimage` runner is the one deliberate exception to "pin the newest Ubuntu LTS" (`ubuntu-22.04`, not `ubuntu-24.04`) — documented inline and in `docs/releasing.md` with the ctx7-sourced rationale (AppImage glibc baseline).
- The tag-ref guard (`if: startsWith(github.ref, 'refs/tags/')`) and the workflow's existing trigger/concurrency shape are unchanged.
- The Flathub manifest is scaffolded only — no attempt to actually open a Flathub PR or submission.

**Ask First:**
- Any change to the `v0.1.0-alpha.*` tag pattern or `cliff.toml`'s `tag_pattern` (out of scope for this story; unchanged).
- Adding a new external crate dependency (none needed; this story is CI/release infra + packaging scaffolding only).

**Never:**
- Do not modify `sprint-status.yaml`.
- Do not commit, push, or open a PR — the orchestrator handles git/PR for this worktree.
- Do not attempt an actual Flathub submission — scaffold the manifest only, per LD-34's "best-effort" language and the explicit task instruction.
- Do not implement the Windows MSI job in this file beyond what Story 6.8 already documented as a future addition.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| `v0.1.0-alpha.x` tag pushed, all secrets configured | tag matches trigger pattern, `macos-dmg` succeeds first | `linux-appimage` builds the AppImage on `ubuntu-22.04`, GPG-signs it via appimagetool, `tauri-action` appends it to the existing draft release; a follow-up step adds `SHA256SUMS` + `SHA256SUMS.asc` | N/A (only verifiable in a real tagged run) |
| `macos-dmg` fails | e.g. a bad Apple secret | `linux-appimage` never starts (`needs: macos-dmg` — GitHub Actions skips dependents of a failed job by default) | no partial/orphaned Linux-only release created |
| `LINUX_GPG_PRIVATE_KEY` missing or not a valid key | import fails or resolves no secret key | "Resolve GPG signing key ID" step fails fast with `::error::` pointing at `docs/releasing.md` | job fails before the build step runs |
| `LINUX_GPG_PASSPHRASE` wrong | passphrase mismatch | `APPIMAGETOOL_FORCE_SIGN=1` makes the tauri-action build step fail outright rather than silently shipping an unsigned AppImage | job fails, no unsigned artifact published |
| `tauri-action` build succeeds but produces no `.AppImage` in `target/release/bundle/appimage/` | unexpected bundler output layout | "Publish checksums + detached signature" step's `find` guard is empty → explicit `::error::` and `exit 1` | job fails, no checksum files uploaded |
| Re-run `linux-appimage` alone (e.g. via "Re-run failed jobs") after `macos-dmg` already succeeded on a prior attempt | draft release already has the DMG (and possibly a stale AppImage from a prior failed attempt) | `tauri-action` finds the existing release by tag and replaces/adds the AppImage asset; `gh release upload ... --clobber` overwrites `SHA256SUMS`/`SHA256SUMS.asc` rather than erroring on "already exists" | idempotent on re-run |
| Both jobs somehow dispatched independently for the same tag (hypothetical — `needs:` should prevent this in the normal push-tag trigger) | race on release creation | Not observable in this sandbox; documented as the specific risk `needs: macos-dmg` exists to close | mitigated by design, not runtime-guarded beyond `needs:` |

</frozen-after-approval>

## Code Map

- `.github/workflows/release.yml` -- MODIFIED. Adds the `linux-appimage` job (`needs: macos-dmg`, `runs-on: ubuntu-22.04`): checkout w/ submodules, apt system deps (webkit2gtk/appindicator/rsvg/patchelf/xdg-utils/build toolchain/gnupg2), Rust toolchain + cache, pnpm/Node, JS deps, GPG key import + key-ID resolution, `tauri-apps/tauri-action@v1` build+sign+publish (appimagetool `SIGN=1`/`SIGN_KEY`/`APPIMAGETOOL_SIGN_PASSPHRASE`/`APPIMAGETOOL_FORCE_SIGN=1`), a checksums+detached-signature step (`SHA256SUMS`/`SHA256SUMS.asc` via `gh release upload`), and a GPG-key cleanup step. Header comment block extended with the new required secrets and the `linux-appimage`-specific runner-pin rationale.
- `packaging/flatpak/com.orgsidian.app.yml` -- NEW. Best-effort Flatpak manifest scaffold (app-id `com.orgsidian.app` matching the Tauri identifier, `org.freedesktop.Platform`/`Sdk` runtime, Wayland/X11/DRI/home `finish-args`). Header comment spells out exactly why it is not submission-ready (repackages a release AppImage instead of building from source; no `cargo-sources.json`; unvalidated AppStream file; placeholder pinned-source hash).
- `packaging/flatpak/com.orgsidian.app.desktop` -- NEW. Minimal `.desktop` entry referenced by the manifest and by a real Flathub submission's AppStream validation.
- `packaging/flatpak/com.orgsidian.app.metainfo.xml` -- NEW. Minimal AppStream metainfo scaffold (summary, MIT/CC0 licensing fields, homepage/bugtracker URLs, empty `<releases>`) — explicitly not run through `appstreamcli validate`.
- `docs/releasing.md` -- MODIFIED. Title/intro broadened to "macOS + Linux"; "What happens on a release tag" now documents both jobs in sequence plus the `needs: macos-dmg` race-avoidance rationale and the `ubuntu-22.04` runner-pin rationale (both ctx7-sourced); the 6.8→6.9→6.10 flow list updated; `LINUX_GPG_PRIVATE_KEY`/`LINUX_GPG_PASSPHRASE` added to the required-secrets table with a "GPG key type matters" callout; a Linux verification recipe added alongside the existing macOS one; a new "Flathub (best-effort, not submitted)" section; "What was validated" and "Known follow-ups" extended for the Linux job and the Flatpak scaffold.

No Rust or TypeScript source changed — this story is CI/release infrastructure + packaging scaffolding + docs only. `crates/orgsidian-shell-app/tauri.conf.json` needed no changes: `bundle.targets: "all"` already includes the `appimage` target on Linux and `icons/icon.png` already exists, so the AppImage target has everything the Tauri bundler needs without any signing material hardcoded in config (GPG signing is supplied purely via `SIGN`/`SIGN_KEY`/`APPIMAGETOOL_SIGN_PASSPHRASE` env vars at release time).

## Tasks & Acceptance

**Execution:**
- [x] `.github/workflows/release.yml`: `linux-appimage` job added (`needs: macos-dmg`, `ubuntu-22.04`, apt deps, GPG import + key-ID resolution, `tauri-action` build/sign/publish, checksums + detached-signature step, GPG cleanup).
- [x] Header comment block extended with `LINUX_GPG_PRIVATE_KEY`/`LINUX_GPG_PASSPHRASE` and the runner-pin note.
- [x] `packaging/flatpak/com.orgsidian.app.yml` + `.desktop` + `.metainfo.xml` best-effort scaffold, with explicit "not submission-ready" documentation.
- [x] `docs/releasing.md` extended: Linux job flow, race-avoidance + runner-pin rationale, secrets table, Linux verification recipe, Flathub best-effort section, validated-vs-not section, known follow-ups.
- [x] YAML validated (`actionlint`, plus a Ruby/YAML parse of the Flatpak manifest since `pyyaml` wasn't available in this sandbox); the metainfo XML validated for well-formedness; `cargo build --workspace --offline` reconfirmed unaffected.

**Acceptance Criteria:**
- Given Story 6.7's release pipeline (extended by Story 6.8 to `release.yml` + a draft-release model), when a release tag is pushed, then `.github/workflows/release.yml` builds the Linux-x86_64 AppImage via the Tauri bundler. *(`linux-appimage` job: `runs-on: ubuntu-22.04` native x86_64, no `--target` flag needed since the host triple `x86_64-unknown-linux-gnu` already matches; `projectPath: crates/orgsidian-shell-app` + system deps wired per the ctx7-verified doc pages. Only exercisable in a real tagged CI run — no Linux runner available in this sandbox.)*
- And the AppImage is GPG-signed with checksums published alongside. *(Embedded appimagetool signing via `SIGN=1`/`SIGN_KEY`/`APPIMAGETOOL_SIGN_PASSPHRASE`/`APPIMAGETOOL_FORCE_SIGN=1` — the ctx7-verified "Distribute → Sign → Linux" env-var contract — plus a `SHA256SUMS` + detached-signed `SHA256SUMS.asc` pair published as additional release assets for tooling-free verification (`sha256sum -c` + `gpg --verify`). Cannot be executed without a real GPG key in this sandbox.)*
- And the artifact is uploaded to the GitHub Release page. *(`tauri-action`'s `releaseDraft: true`/`prerelease: true` with `tagName` left unset — same pattern as `macos-dmg` — appends the AppImage to the release for `github.ref_name`; `needs: macos-dmg` guarantees that release already exists as a draft by the time this job runs, so no competing release is created.)*
- And a Flathub manifest is filed best-effort per LD-34. *(`packaging/flatpak/com.orgsidian.app.yml` + `.desktop` + `.metainfo.xml` scaffolded; NOT submitted to Flathub — `docs/releasing.md`'s "Flathub (best-effort, not submitted)" section documents exactly what remains before a real submission, per the task's explicit "do NOT attempt to submit to Flathub" instruction.)*

## Design Notes

- **Why a sibling job in the same file, not a new workflow.** The task and this story's own framing ("EXTENDS the release pipeline Story 6.8 created") call for one release workflow per tag, not a second workflow racing or duplicating trigger logic. Matches the existing `macos-dmg` job's shape (checkout → toolchain/cache → JS deps → credential import/resolve → `tauri-action` → cleanup) so a maintainer reading the file sees one consistent pattern per platform.
- **Why `needs: macos-dmg` instead of two independent parallel jobs.** `tauri-apps/tauri-action` gets-or-creates the release for the pushed tag on every invocation; two jobs hitting "create" simultaneously on the very first run for a tag is a real race with no graceful merge on GitHub's side (the loser's create call is rejected outright). Serializing removes the race entirely at the cost of roughly doubling this workflow's wall-clock time — judged acceptable for an event that fires a handful of times per milestone, not on any inner-loop path. The alternative (having `macos-dmg`'s first step create an empty release, then fan both platform builds out in parallel against that already-existing release) is documented as a `docs/releasing.md` follow-up rather than implemented now, since the task's explicit warning is "watch out for two jobs racing" and this satisfies that without adding a second axis of workflow restructuring beyond this story's scope.
- **Why `ubuntu-22.04`, not this repo's usual `ubuntu-24.04` pin.** This is the one deliberate exception to `pr.yml`/`nightly.yml`/`homebrew-cask.yml`'s convention. The ctx7-verified Tauri 2.x "Distribute → AppImage" Limitations section is explicit: build on the oldest base system you intend to support, because a newer glibc raises the minimum glibc version the resulting binary needs at runtime — exactly backwards from what an AppImage exists to promise. Ubuntu 22.04 is the oldest still-supported release providing `libwebkit2gtk-4.1-dev` (Tauri v2's WebKitGTK requirement) from its normal repositories, making it the correct baseline rather than an arbitrarily conservative pin.
- **Why appimagetool's built-in GPG signing (`SIGN=1`) rather than only a detached signature over the raw AppImage.** The task's ctx7-verification instruction turned up a first-class, Tauri-documented mechanism for this ("Distribute → Sign → Linux") — using it directly satisfies "the AppImage is GPG-signed" via the officially supported path instead of a hand-rolled equivalent. The additional `SHA256SUMS`/`SHA256SUMS.asc` pair is added on top (not instead) because the embedded signature is only checkable with appimagetool itself (`--appimage-signature`); a `sha256sum -c` + `gpg --verify` pair lets an adopter verify the download with tools every Linux distro already ships, at zero extra secret cost (same imported key, same passphrase).
- **Why the Flathub manifest is honestly scoped as non-submittable.** LD-34 says "filed best-effort," and the task is explicit: do not attempt to actually submit. Rather than producing a manifest that looks complete but would fail Flathub's review bot on the very first automated check (network access during build, no `cargo-sources.json`), the scaffold's own header comment enumerates every concrete gap a maintainer must close first. This is judged more useful than a manifest that appears ready and quietly isn't.
- **Honest scope boundary.** Nothing in this story could be exercised end-to-end in this sandbox: no Linux GitHub Actions runner, no GPG signing key, and no real tagged release exist here. What's shipped is a workflow whose shape, system-dependency list, and signing env-var contract are verified against current upstream Tauri docs (not training-data recall), fail-fast error messages at every point a misconfigured secret or unexpected bundler layout would otherwise cause a confusing downstream failure, and a `docs/releasing.md` section spelling out exactly what a maintainer must still confirm on the first real tagged run.

## Verification

**Commands:**
- `git submodule update --init --recursive` -- expected: `tree-sitter-org` submodule present (already initialized in this worktree from Story 6.8's verification).
- `actionlint .github/workflows/release.yml` -- expected: no output (clean).
- `ruby -ryaml -e "YAML.load_file('packaging/flatpak/com.orgsidian.app.yml')"` -- expected: parses without error (used in place of `python3 -c "import yaml..."` since `pyyaml` was not installed in this sandbox; Ruby's stdlib `YAML`/Psych was available instead).
- `python3 -c "import xml.dom.minidom as m; m.parse('packaging/flatpak/com.orgsidian.app.metainfo.xml')"` -- expected: well-formed XML.
- `cargo build --workspace --offline` -- expected: builds clean (this story touches no Rust source; confirms nothing else broke).

**Result (2026-09-05):** `actionlint` clean on the modified `.github/workflows/release.yml` (both `macos-dmg` and the new `linux-appimage` job). No standalone shell scripts were added (`linux-appimage`'s shell steps are inline in the workflow YAML, already covered by actionlint's embedded shellcheck integration, which passed clean), so `shellcheck` had nothing additional to run against. The Flatpak manifest parsed successfully via Ruby's YAML library; the metainfo file parsed as well-formed XML via Python's `xml.dom.minidom`. `cargo build --workspace --offline` finished successfully (all workspace crates) after re-initializing the `tree-sitter-org` submodule — no regressions, since this story touches only `.github/workflows/`, `packaging/flatpak/`, and `docs/`. GPG signing, the actual AppImage build, and the release append are **not validated** here — they require a real Linux GitHub Actions runner and a real GPG signing key, neither of which exists in this sandbox; they can only be confirmed by a maintainer pushing a real `v0.1.0-alpha.x` tag with `LINUX_GPG_PRIVATE_KEY`/`LINUX_GPG_PASSPHRASE` configured per `docs/releasing.md`.

## Spec Change Log

- 2026-09-05 — Implemented. `.github/workflows/release.yml` (`linux-appimage` job added, `needs: macos-dmg`), `packaging/flatpak/com.orgsidian.app.yml` + `.desktop` + `.metainfo.xml` (best-effort scaffold, not submitted), `docs/releasing.md` (Linux section, secrets table rows, Flathub best-effort section, validated-vs-not, known follow-ups). Workflow shape, system deps, and GPG signing env-var contract verified against current Tauri 2.x docs via `ctx7` (not guessed). Release-append race avoided via `needs: macos-dmg` rather than independent parallel jobs. GPG signing and the actual Linux build unverifiable offline — documented as the explicit decision-grade handoff in `docs/releasing.md`. Status → review.
