# Releasing Orgsidian (macOS)

> Companion to [`.github/workflows/release.yml`](../.github/workflows/release.yml),
> [`.github/workflows/homebrew-cask.yml`](../.github/workflows/homebrew-cask.yml),
> and [`scripts/publish-homebrew-cask.sh`](../scripts/publish-homebrew-cask.sh).
> Covers Story 6.8 (macOS DMG packaging + signing + notarization, LD-19 +
> LD-34). Story 6.9 (Linux AppImage) and the Windows MSI story will extend
> this doc with their own sections when they land.

## What happens on a release tag

Pushing a tag matching `v0.1.0-alpha.*` (the LD-33 `v*` tag scheme — kept in
lockstep with `cliff.toml`'s `tag_pattern`) triggers `release.yml`'s
**`macos-dmg`** job (runs on `macos-14`, and only for an actual tag push —
guarded with `if: startsWith(github.ref, 'refs/tags/')` so a stray
`workflow_dispatch` against a branch can't create a bogus release). It
builds the app for `aarch64-apple-darwin`, signs it with the Apple
Developer ID Application certificate, notarizes it via `notarytool` (staple
attached — this is the Tauri bundler's default behavior once
`APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` are present), and creates a
**draft, prerelease** GitHub Release for the tag with the signed `.dmg`
attached.

The GitHub Release is created as a **draft** deliberately — a maintainer
reviews the notes and artifact, then publishes it. Story 6.10 owns the
actual "go public" announcement step.

## The 6.8 -> 6.9 -> 6.10 -> cask flow

The Homebrew cask is published by a **separate** workflow,
[`homebrew-cask.yml`](../.github/workflows/homebrew-cask.yml), triggered on
`on: release: types: [published]` — it does **not** run as a job chained
directly after `macos-dmg`. This matters because `macos-dmg` creates the
release as a draft, and a draft release's asset URLs
(`releases/download/<tag>/<asset>`) 404 for the unauthenticated fetch that
`brew install` performs — exactly the URL the cask's `url` stanza points at.
Publishing the cask right after `macos-dmg` would therefore write out a
cask that 404s until someone happens to publish the release later; waiting
for the `published` event instead means the cask job only ever runs once
that asset URL is actually live.

The full sequence, end to end:

1. **Story 6.8** (`release.yml`, job `macos-dmg`) — tag push builds, signs,
   notarizes, and creates the **draft** release with the `.dmg` attached.
2. **Story 6.9** (`release.yml`, sibling job) — adds the Linux AppImage to
   the same draft release.
3. **Story 6.10** (maintainer, manual) — reviews the draft release and
   publishes it (GitHub UI "Publish release", or
   `gh release edit <tag> --draft=false`). This is what fires the
   `release: published` event.
4. **`homebrew-cask.yml`** (job `publish-cask`, runs on `ubuntu-24.04`) —
   triggered by that `published` event. Resolves the tag/version from
   `github.event.release.tag_name`, downloads the `.dmg` asset from the now
   -public release, computes its sha256, renders
   [`packaging/homebrew/orgsidian.rb.tmpl`](../packaging/homebrew/orgsidian.rb.tmpl),
   and pushes the result to `Casks/orgsidian.rb` in the external
   `orgsidian/tap` repository.

Step 4 is **idempotent**: `scripts/publish-homebrew-cask.sh` stages the
rendered cask and diffs it against the tap's current `Casks/orgsidian.rb`
before committing — if nothing changed (e.g. the job is re-run for the same
release), it's a no-op rather than an empty commit.

Note that only the draft-to-published transition fires `release: published`
— editing an already-published release again does not refire it. If a cask
ever needs to be re-rendered without a fresh draft-to-published transition
(e.g. the DMG asset on an existing published release was replaced by hand),
re-running the `publish-cask` job from the Actions UI ("Re-run jobs" on the
existing `homebrew-cask.yml` run) is the workaround for now; this workflow
has no `workflow_dispatch` trigger of its own, which is an acceptable gap
until that scenario actually comes up in practice.

## Required repository secrets

Configure these under **Settings → Secrets and variables → Actions** on
`orgsidian/orgsidian` before the first tag push. None of them can be
guessed or defaulted — the workflow fails fast (with a pointer back to this
doc) if any signing/notarization secret is missing or wrong.

| Secret | What it is | How to get it |
|---|---|---|
| `APPLE_CERTIFICATE` | base64 of the exported Developer ID Application `.p12` | Export the cert + private key from Keychain Access as a `.p12`, then `base64 -i cert.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | the password you set when exporting that `.p12` | chosen at export time |
| `KEYCHAIN_PASSWORD` | password for the throwaway keychain the CI job creates | not an Apple secret — generate any random string once (e.g. `openssl rand -base64 32`) and store it |
| `APPLE_ID` | Apple ID email used to notarize | the Apple Developer account's login email |
| `APPLE_PASSWORD` | an **app-specific password** for that Apple ID | generate at [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords — **not** the account password |
| `APPLE_TEAM_ID` | 10-character Apple Developer Team ID | [developer.apple.com account → Membership](https://developer.apple.com/account/#/membership) |
| `HOMEBREW_TAP_GITHUB_TOKEN` | fine-grained PAT, `contents:write` scoped to `orgsidian/tap` only | mint under the account that owns/administers `orgsidian/tap`; `GITHUB_TOKEN` cannot push to a different repository |

`GITHUB_TOKEN` (built into every Actions run — no setup needed) is what
creates the release and uploads the DMG asset to `orgsidian/orgsidian`
itself in `release.yml`; the separate `homebrew-cask.yml` workflow uses its
own run's `GITHUB_TOKEN` to read that (by-then-published) release back.

**Certificate type matters.** The imported cert must resolve to a
**"Developer ID Application"** identity under
`security find-identity -v -p codesigning` — the workflow greps for exactly
that string and fails with a clear error if it isn't found (a "Mac
Developer" / "Apple Development" cert, used for local Xcode runs, will not
notarize).

## Manual bootstrap this workflow does NOT do

- **Creating `orgsidian/tap` itself.** The publish script assumes the repo
  already exists with a `Casks/` directory and a default branch it can push
  to. Create it once, by hand, before the first release tag.
- **Minting `HOMEBREW_TAP_GITHUB_TOKEN`.** Same bootstrap step — a PAT has
  to be created by a human with write access to that repo.
- **Publishing the draft GitHub Release.** The workflow leaves it as a
  draft; a maintainer reviews and clicks "Publish" (or uses `gh release
  edit <tag> --draft=false`).

## Verifying a release locally (maintainer, after signing is set up)

```bash
# Mount the DMG first — codesign/spctl need the .app extracted, not the
# disk image itself:
hdiutil attach /path/to/Orgsidian_<version>_aarch64.dmg
# then, against the mounted copy (path shown by `hdiutil attach`, typically
# /Volumes/Orgsidian/Orgsidian.app):

# Confirm the app is signed:
codesign -dv --verbose=4 /Volumes/Orgsidian/Orgsidian.app

# Confirm Gatekeeper accepts it (notarization + staple):
spctl -a -vv -t install /Volumes/Orgsidian/Orgsidian.app

# Confirm the staple ticket, against the .dmg itself (not the mounted app):
stapler validate /path/to/Orgsidian_<version>_aarch64.dmg

# then detach:
hdiutil detach /Volumes/Orgsidian
```

## What was validated without real Apple / tap credentials

This story's implementation was scaffolded and YAML-validated in a sandbox
with no Apple Developer ID certificate, no Apple ID, and no
`orgsidian/tap` PAT available. The workflow shape and secret names were
verified against the current Tauri 2.x documentation (`tauri-apps/tauri-docs`,
"Distribute → Sign → macOS" and "Distribute → Pipelines → GitHub", fetched
2026-09-05). What can only be confirmed by a maintainer running a real
tagged release:

- the certificate import + `security find-identity` grep actually resolves
  a real "Developer ID Application" identity;
- `notarytool` actually accepts the `APPLE_ID`/`APPLE_PASSWORD`/
  `APPLE_TEAM_ID` triple and notarization + stapling succeed end-to-end;
- the DMG asset name the Tauri bundler produces matches what
  `scripts/publish-homebrew-cask.sh` expects (`*.dmg` glob — the script
  does not hardcode a filename precisely to avoid this class of drift);
- the `orgsidian/tap` push succeeds with the real PAT and Homebrew accepts
  the rendered cask (`brew audit --cask orgsidian` / `brew install
  --cask orgsidian/tap/orgsidian` from a real tap checkout).

## Known follow-ups (documented, not yet acted on)

- **Manual keychain import may be partly redundant.** `release.yml`'s
  "Import Apple Developer ID certificate" step imports the cert into
  `build.keychain` itself (to resolve `CERT_ID` for
  `APPLE_SIGNING_IDENTITY`), but `tauri-apps/tauri-action` also imports
  `APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD` on its own when those env
  vars are present on its step. The two imports target the cert
  independently and haven't been proven to interact cleanly end-to-end (vs.
  just each working in isolation) — **verify signing on the first real
  tagged run** and simplify if the manual import turns out to be pure
  duplication.
- **PAT-in-clone-URL.** `scripts/publish-homebrew-cask.sh` clones
  `orgsidian/tap` via `https://x-access-token:${HOMEBREW_TAP_GITHUB_TOKEN}@github.com/...`.
  GitHub Actions masks the secret value in logs, so this is safe as
  configured, but the token still transiently exists in the process's
  argv/URL rather than a header. The stricter alternative — configuring
  `http.extraheader` with the token as a bearer `Authorization` header
  (`git -c http.extraheader="AUTHORIZATION: basic $(printf 'x-access-token:%s' "$HOMEBREW_TAP_GITHUB_TOKEN" | base64)" clone ...`)
  — is optional hardening, not applied here.
- **`homebrew-cask.yml` has no manual re-run trigger.** See "The 6.8 -> 6.9
  -> 6.10 -> cask flow" above — a re-render outside the draft-to-published
  transition currently means re-running the existing job from the Actions
  UI; a `workflow_dispatch` input (tag name) would be a small follow-up if
  that becomes a recurring need.
