#!/usr/bin/env bash
# publish-homebrew-cask.sh — Story 6.8 (LD-34 distribution channels).
#
# Renders packaging/homebrew/orgsidian.rb.tmpl for a published macOS-arm64
# DMG release asset and pushes the result to `Casks/orgsidian.rb` in the
# external `orgsidian/tap` repository. Invoked by the `publish-cask` job in
# .github/workflows/homebrew-cask.yml, which triggers on `release:
# published` — i.e. only once a maintainer (Story 6.10) has flipped the
# draft release that release.yml's `macos-dmg` job created into a real,
# public release. This deliberately does NOT run chained directly after
# `macos-dmg`: while the release is still a draft, the asset's public
# download URL 404s, which would produce a cask that can't actually be
# installed.
#
# Required environment:
#   GH_TOKEN                    - token with read access to
#                                  orgsidian/orgsidian (built-in
#                                  secrets.GITHUB_TOKEN suffices; the repo
#                                  is public and the release is published
#                                  by the time this script runs).
#   HOMEBREW_TAP_GITHUB_TOKEN    - PAT (fine-grained, contents:write scoped
#                                  to orgsidian/tap only) — GITHUB_TOKEN
#                                  cannot push to a different repository.
#   RELEASE_TAG                  - the tag of the published release
#                                  (e.g. v0.1.0-alpha.3); homebrew-cask.yml
#                                  passes `github.event.release.tag_name`.
#
# Not runnable end-to-end outside a real tagged CI run (no Apple Developer
# ID cert / no orgsidian/tap PAT exist in this sandbox) — see
# docs/releasing.md "Required repository secrets" for the full list and
# "What this script does NOT do" below for the manual bootstrap step it
# depends on.

set -euo pipefail

REPO="orgsidian/orgsidian"
TAP_REPO="orgsidian/tap"
TEMPLATE="packaging/homebrew/orgsidian.rb.tmpl"

: "${GH_TOKEN:?GH_TOKEN is required (repo-read access to ${REPO})}"
: "${HOMEBREW_TAP_GITHUB_TOKEN:?HOMEBREW_TAP_GITHUB_TOKEN is required (contents:write PAT scoped to ${TAP_REPO})}"
: "${RELEASE_TAG:?RELEASE_TAG is required (e.g. v0.1.0-alpha.3)}"

if [ ! -f "$TEMPLATE" ]; then
  echo "::error::${TEMPLATE} not found — run this script from the repo root."
  exit 1
fi

VERSION="${RELEASE_TAG#v}"

echo "Looking up the DMG asset on release ${RELEASE_TAG} (${REPO})..."
ASSET_JSON="$(gh release view "$RELEASE_TAG" --repo "$REPO" --json assets \
  --jq '.assets[] | select(.name | endswith(".dmg"))')"

if [ -z "$ASSET_JSON" ]; then
  echo "::error::No .dmg asset found on release ${RELEASE_TAG}. Did the macos-dmg job finish successfully?"
  exit 1
fi

ASSET_NAME="$(echo "$ASSET_JSON" | jq -r '.name')"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${ASSET_NAME}"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

echo "Downloading ${ASSET_NAME}..."
gh release download "$RELEASE_TAG" --repo "$REPO" --pattern '*.dmg' --dir "$WORKDIR" --clobber

DMG_PATH="$WORKDIR/$ASSET_NAME"
# sha256sum (coreutils) is guaranteed on the ubuntu-24.04 runner this job
# uses; shasum is only incidentally present via perl.
SHA256="$(sha256sum "$DMG_PATH" | awk '{print $1}')"

echo "Rendering cask (version=${VERSION}, sha256=${SHA256})..."
RENDERED="$WORKDIR/orgsidian.rb"
sed \
  -e "s|__VERSION__|${VERSION}|g" \
  -e "s|__URL__|${DOWNLOAD_URL}|g" \
  -e "s|__SHA256__|${SHA256}|g" \
  "$TEMPLATE" > "$RENDERED"

echo "Cloning ${TAP_REPO}..."
TAP_DIR="$WORKDIR/tap"
git clone --depth 1 "https://x-access-token:${HOMEBREW_TAP_GITHUB_TOKEN}@github.com/${TAP_REPO}.git" "$TAP_DIR"

mkdir -p "$TAP_DIR/Casks"
cp "$RENDERED" "$TAP_DIR/Casks/orgsidian.rb"

cd "$TAP_DIR"
git config user.name "orgsidian-release-bot"
git config user.email "releases@orgsidian.app"

# Stage first, then diff against the index: `git diff` (working tree vs.
# index) ignores untracked files, so on the FIRST publish — when
# Casks/orgsidian.rb does not yet exist in the tap — the new file would be
# untracked and an unstaged `git diff --quiet` would wrongly report "no
# change" and skip the push. Staging then checking `--cached` catches both
# a brand-new file and a modified one.
git add Casks/orgsidian.rb
if git diff --cached --quiet; then
  echo "Casks/orgsidian.rb already up to date for ${RELEASE_TAG} — nothing to push."
  exit 0
fi

git commit -m "chore(cask): update orgsidian to ${RELEASE_TAG}"
git push origin HEAD

echo "Published Casks/orgsidian.rb (${VERSION}) to ${TAP_REPO}."
