# Releasing Orgsidian (macOS + Linux)

> Companion to [`.github/workflows/release.yml`](../.github/workflows/release.yml),
> [`.github/workflows/homebrew-cask.yml`](../.github/workflows/homebrew-cask.yml),
> [`scripts/publish-homebrew-cask.sh`](../scripts/publish-homebrew-cask.sh),
> and [`packaging/flatpak/com.orgsidian.app.yml`](../packaging/flatpak/com.orgsidian.app.yml).
> Covers Story 6.8 (macOS DMG packaging + signing + notarization) and
> Story 6.9 (Linux AppImage packaging + GPG signing + best-effort Flathub
> manifest) — both LD-19 (signing) + LD-34 (distribution channels). The
> Windows MSI story will extend this doc with its own section when it
> lands.

## What happens on a release tag

Pushing a tag matching `v0.1.0-alpha.*` (the LD-33 `v*` tag scheme — kept in
lockstep with `cliff.toml`'s `tag_pattern`) triggers `release.yml`'s two
jobs, in order:

1. **`macos-dmg`** (runs on `macos-14`, and only for an actual tag push —
   guarded with `if: startsWith(github.ref, 'refs/tags/')` so a stray
   `workflow_dispatch` against a branch can't create a bogus release). It
   builds the app for `aarch64-apple-darwin`, signs it with the Apple
   Developer ID Application certificate, notarizes it via `notarytool`
   (staple attached — this is the Tauri bundler's default behavior once
   `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` are present), and creates a
   **draft, prerelease** GitHub Release for the tag with the signed `.dmg`
   attached.
2. **`linux-appimage`** (`needs: macos-dmg`, runs on `ubuntu-22.04` — see
   "Why `ubuntu-22.04`, not `ubuntu-24.04`" below). It builds the AppImage
   for `x86_64-unknown-linux-gnu` (the runner's native target — no
   `--target` flag needed), GPG-signs it via appimagetool's built-in
   signing (`SIGN=1`/`SIGN_KEY`/`APPIMAGETOOL_SIGN_PASSPHRASE`), appends it
   to the **same** draft release `macos-dmg` created, then publishes a
   `SHA256SUMS` + detached-signed `SHA256SUMS.asc` pair as two more release
   assets.

The GitHub Release is created as a **draft** deliberately — a maintainer
reviews the notes and both artifacts, then publishes it. Story 6.10 owns
the actual "go public" announcement step.

### Why `linux-appimage` runs after `macos-dmg`, not in parallel

`tauri-apps/tauri-action` looks up the GitHub Release for the pushed tag
and creates it if it doesn't exist yet. If both jobs ran as independent
parallel jobs on the same tag, there's a real race the first time a tag is
pushed: both could see "no release yet" at the same moment and both try to
create one, and GitHub's API rejects the loser of that race outright rather
than merging the two attempts — that job then fails for a reason that has
nothing to do with its own build. Making `linux-appimage` declare
`needs: macos-dmg` removes the race entirely: `macos-dmg` always creates
the release first, and `linux-appimage`'s `tauri-action` step finds that
same (still-draft) release by tag and appends to it. The tradeoff is that
the two builds run sequentially instead of in parallel (roughly doubling
this workflow's wall-clock time); that's judged worth it for a release
workflow that only runs a handful of times per milestone, versus adding a
more elaborate locking scheme for a job that isn't on any developer's
inner-loop critical path.

### Why `ubuntu-22.04`, not `ubuntu-24.04`, for the AppImage build

Every other `ubuntu-*` job in this repo (`pr.yml`, `nightly.yml`,
`homebrew-cask.yml`) pins `ubuntu-24.04` per
[[feedback_version_policy]]'s "pin the newest LTS" default.
`linux-appimage` is a deliberate, documented exception. Per the Tauri 2.x
docs' AppImage "Limitations" section (ctx7-verified 2026-09-05): an
AppImage must be built on the **oldest** base system you intend to support,
because glibc's symbol versioning only grows over time — a binary built on
a newer glibc raises the minimum glibc version end users need to run it at
all (the classic `/usr/lib/libc.so.6: version 'GLIBC_2.33' not found`
failure on an older distro), which defeats the entire "one binary runs
everywhere" premise of shipping an AppImage in the first place. Ubuntu
22.04 is the oldest still-supported Ubuntu release that provides Tauri
v2's required `libwebkit2gtk-4.1-dev` from its normal apt repositories, so
it's the correct baseline for this one job, not merely an older pin chosen
out of caution.

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
2. **Story 6.9** (`release.yml`, job `linux-appimage`, `needs: macos-dmg`)
   — builds, GPG-signs, and adds the Linux AppImage + `SHA256SUMS` +
   `SHA256SUMS.asc` to the same draft release.
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
| `LINUX_GPG_PRIVATE_KEY` | base64 of the exported ASCII-armored GPG private key used to sign the AppImage | generate with `gpg2 --full-gen-key`, then `gpg --export-secret-keys --armor <key-id> \| base64 \| pbcopy` (or `base64 -w0` on Linux) |
| `LINUX_GPG_PASSPHRASE` | passphrase for that GPG key | chosen when generating the key |
| `HOMEBREW_TAP_GITHUB_TOKEN` | fine-grained PAT, `contents:write` scoped to `orgsidian/tap` only | mint under the account that owns/administers `orgsidian/tap`; `GITHUB_TOKEN` cannot push to a different repository |

`GITHUB_TOKEN` (built into every Actions run — no setup needed) is what
creates the release and uploads the DMG/AppImage/checksum assets to
`orgsidian/orgsidian` itself in `release.yml`; the separate
`homebrew-cask.yml` workflow uses its own run's `GITHUB_TOKEN` to read that
(by-then-published) release back.

**GPG key type matters.** Generate a dedicated signing-only key for this
purpose (do not reuse a maintainer's personal identity key) so the private
key material stored in `LINUX_GPG_PRIVATE_KEY` can be rotated independently
of anyone's personal GPG identity. `linux-appimage` imports it into a
throwaway `GNUPGHOME` per run and deletes it again in an `if: always()`
cleanup step, the same "import, use, tear down" shape as `macos-dmg`'s
throwaway keychain.

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
- **Generating the `LINUX_GPG_PRIVATE_KEY`/`LINUX_GPG_PASSPHRASE` signing
  key.** Same class of bootstrap step as the Apple secrets — a maintainer
  runs `gpg2 --full-gen-key` once, keeps the private key safe, and stores
  the two secrets.
- **Filing the actual Flathub submission.** See "Flathub (best-effort, not
  submitted)" below — this story ships a manifest scaffold only.

## Verifying a release locally (maintainer, after signing is set up)

### macOS (DMG)

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

### Linux (AppImage)

```bash
# Confirm the checksum matches the published SHA256SUMS:
sha256sum -c SHA256SUMS --ignore-missing

# Confirm SHA256SUMS itself is authentic (import the maintainer's public
# key first — see the release notes / a future docs/verifying-releases.md
# for the key's fingerprint once one is published):
gpg --verify SHA256SUMS.asc SHA256SUMS

# Confirm the AppImage's own embedded appimagetool signature (requires the
# same public key imported above):
chmod +x Orgsidian_<version>_amd64.AppImage
./Orgsidian_<version>_amd64.AppImage --appimage-signature
```

## Flathub (best-effort, not submitted)

Per LD-34 ("a Flathub manifest is filed best-effort"), this story scaffolds
a Flatpak manifest at
[`packaging/flatpak/com.orgsidian.app.yml`](../packaging/flatpak/com.orgsidian.app.yml)
(plus a minimal `.desktop` file and AppStream `metainfo.xml` alongside it)
but does **not** open a Flathub submission — that requires infrastructure
this story deliberately does not build:

- **A from-source build, not a repackaged AppImage.** Flathub's review
  process rejects manifests that merely unpack another distribution format
  (AppImage, `.deb`, etc.) inside the sandbox; the scaffolded
  `build-commands` do exactly that as a placeholder and are called out as
  such in the manifest's own header comment.
- **Network-isolated builds.** Flathub's build bot disallows network access
  during the build itself, so every Rust crate and JS package needs to be
  pre-declared as a `sources:` entry — crates via a `cargo-sources.json`
  generated with Flathub's `flatpak-cargo-generator.py`, JS deps via a
  comparable offline mirror. Neither is generated here.
- **A validated AppStream file.** `com.orgsidian.app.metainfo.xml` has not
  been run through `appstreamcli validate` or reviewed for Flathub's
  content guidelines (real screenshots, a fuller `<description>`, per
  -version `<release>` entries).
- **A pinned, hashed release source.** The manifest's `sources:` entry
  points at a `releases/latest/download/...` URL with a placeholder
  `sha256`, which is not how Flathub expects a release to be pinned (a
  real submission pins an exact tag + a computed hash, updated by
  Flathub's own version-bump tooling on every release).

A maintainer picking this up for a real Flathub submission should treat
the scaffold as a starting skeleton (app-id, `finish-args`, desktop
metadata shape), not as something close to submission-ready.

## What was validated without real Apple / GPG / tap credentials

This story's implementation was scaffolded and YAML-validated in a sandbox
with no Apple Developer ID certificate, no Apple ID, no GPG signing key,
and no `orgsidian/tap` PAT available. The workflow shape and secret names
were verified against the current Tauri 2.x documentation
(`tauri-apps/tauri-docs`, "Distribute → Sign → macOS", "Distribute → Sign →
Linux", "Distribute → Pipelines → GitHub", "Start → Prerequisites", and
"Distribute → AppImage", fetched 2026-09-05). What can only be confirmed by
a maintainer running a real tagged release:

- the certificate import + `security find-identity` grep actually resolves
  a real "Developer ID Application" identity;
- `notarytool` actually accepts the `APPLE_ID`/`APPLE_PASSWORD`/
  `APPLE_TEAM_ID` triple and notarization + stapling succeed end-to-end;
- the DMG asset name the Tauri bundler produces matches what
  `scripts/publish-homebrew-cask.sh` expects (`*.dmg` glob — the script
  does not hardcode a filename precisely to avoid this class of drift);
- the `orgsidian/tap` push succeeds with the real PAT and Homebrew accepts
  the rendered cask (`brew audit --cask orgsidian` / `brew install
  --cask orgsidian/tap/orgsidian` from a real tap checkout);
- the `linux-appimage` job's apt package list actually resolves on
  `ubuntu-22.04` and produces a working AppImage (no Linux GitHub Actions
  runner was available to exercise this job in the sandbox);
- `LINUX_GPG_PRIVATE_KEY` import + `gpg --list-secret-keys` key-ID
  resolution work against a real exported key, and appimagetool's
  `SIGN=1`/`SIGN_KEY`/`APPIMAGETOOL_SIGN_PASSPHRASE` contract actually
  embeds a verifiable signature (`--appimage-signature`) rather than
  failing closed via `APPIMAGETOOL_FORCE_SIGN=1`;
- the detached `SHA256SUMS.asc` signature verifies with `gpg --verify`
  against the same public key end-to-end;
- `needs: macos-dmg` actually serializes the two jobs as intended and
  `linux-appimage`'s `tauri-action` step finds (rather than re-creates) the
  draft release `macos-dmg` produced.

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
- **`macos-dmg` and `linux-appimage` build sequentially, not in parallel.**
  See "Why `linux-appimage` runs after `macos-dmg`, not in parallel" above
  — this is a deliberate tradeoff (correctness over wall-clock time) rather
  than an oversight; revisit only if release wall-clock time becomes a real
  pain point (e.g. by having `macos-dmg` create the release with no
  platform asset yet in a dedicated first step, then fanning both platform
  builds out in parallel behind that).
- **The Flathub manifest is a scaffold, not a submission.** See "Flathub
  (best-effort, not submitted)" above for the concrete gaps (from-source
  build, `cargo-sources.json`, AppStream validation, pinned release
  hashes) a maintainer needs to close before actually opening a Flathub
  PR.
