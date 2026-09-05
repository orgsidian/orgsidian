# Orgsidian

[![Repo](https://img.shields.io/badge/repo-orgsidian%2Forgsidian-181717?logo=github)](https://github.com/orgsidian/orgsidian) ![Status](https://img.shields.io/badge/v0.1%20Alpha-code--complete-yellow) ![License](https://img.shields.io/badge/license-MIT-blue)

> The cross-platform desktop **planner-and-knowledge** app that opens, edits, and organizes `.org` files (Emacs org-mode format) — without requiring Emacs. Local-first, OSS, format-faithful.

**Status: v0.1 Alpha — code-complete, release tag pending.** This repository is public. The v0.1 Alpha release (signed macOS DMG, Homebrew cask, Linux AppImage) is built and published from a `v0.1.0-alpha.*` tag per [`docs/releasing.md`](./docs/releasing.md); once that tag is pushed, the signed builds appear on the [Releases page](https://github.com/orgsidian/orgsidian/releases).

---

## Why

Org-mode is the most powerful integrated planner-and-knowledge format ever shipped, but it lives inside Emacs — and Emacs is a hard barrier for most people. Orgsidian is *desktop-native, cross-platform, OSS, and faithful to the format*, treating tasks, time, and notes as peers on a unified surface instead of bolting a task list onto a note-taking app (or vice versa).

The marketing wedge is **task-first** — "planner powered by org-mode" — but the product underneath is the integration itself: one plain-text format, read and written byte-for-byte round-trip faithfully, driving both a daily planner and a knowledge base.

## Install

v0.1 Alpha ships prebuilt binaries for macOS and Linux (Windows lands later — see [Roadmap](#roadmap)). Full build details, required secrets, and verification steps are documented in [`docs/releasing.md`](./docs/releasing.md); the short version:

| Platform | Path | Notes |
|---|---|---|
| **macOS** (Apple Silicon) | Download the signed + notarized `.dmg` from the [latest Release](https://github.com/orgsidian/orgsidian/releases/latest) | Opens without a Gatekeeper warning — no manual "Open Anyway" needed. |
| **macOS** (Homebrew) | `brew install --cask orgsidian/tap/orgsidian` | Cask formula published to [`orgsidian/tap`](https://github.com/orgsidian/tap) once each release is published. |
| **Linux** (x86_64) | Download the `.AppImage` from the [latest Release](https://github.com/orgsidian/orgsidian/releases/latest), `chmod +x`, then run it | GPG-signed; `SHA256SUMS` + `SHA256SUMS.asc` are published alongside for verification (see `docs/releasing.md` § "Verifying a release locally"). A best-effort Flathub manifest scaffold also lives at [`packaging/flatpak/`](./packaging/flatpak/) (not yet submitted to Flathub). |

Building from source (any platform) works the same way as [Development](#development) below — `pnpm tauri dev` for a dev build, `pnpm tauri build` for a local release-mode bundle.

## v0.1 Alpha feature summary

v0.1 Alpha is a **first-launch-to-first-week** slice: install it, open a Vault, and see your day and week without ever touching a config file.

- **Parser & round-trip fidelity** — a `tree-sitter-org`-backed parser with a byte-identical round-trip serializer, gated by a CI corpus of real-world `.org` files plus an Emacs ground-truth oracle. `orgsidian parse <file>` is available as a CLI smoke command.
- **Vault & SQLite index** — designate any folder as a Vault; an initial scan builds a local SQLite index (progress UI on first scan), rebuildable at any time by deleting the index file and relaunching.
- **Editor surface** — a CodeMirror 6 editor with Raw / Pseudo-WYSIWYG / Split modes (remembered per file), heading/TODO/tag/timestamp/checkbox decorations, clickable links, Schedule/Deadline editing with a date picker, cross-platform keybindings, and an opt-in [Emacs keybindings mode](./docs/user-guide/emacs-keybindings.md).
- **External-edit safety** — external changes to a file you have open are watched continuously: a clean (unedited) buffer auto-reloads and re-indexes; a dirty (unsaved) buffer blocks Save with a conflict banner instead of silently overwriting or losing either version. (The full interactive Merge Dialog is a later milestone — see [Roadmap](#roadmap).)
- **Starter Vaults** — first launch offers to generate a small, realistic [Starter Vault](./docs/user-guide/starter-vaults.md) (**Personal GTD** or **Student**) with Inbox/Project/Journal/Someday content already scheduled across the coming week, so Today and Week Agenda aren't empty on first open.
- **Today + Week Agenda** — `/today` shows Scheduled-today and Deadline-today-or-overdue items across the whole Vault; `/agenda/week` widens that to a rolling 7-day view. Both link straight back to the source Headline in the editor.
- **Dark + light themes** — WCAG AA–contrast dark and light themes with an instant Settings → Appearance toggle (Light / Dark / follow-system).
- **First-run coaching** — a non-modal first-run coaching balloon points out the Today view (click any task to open its source file) so a first-time user isn't left guessing.
- **Frozen index query API** — the `IndexQuery` trait (`crates/orgsidian-index/src/query/`) is frozen as a stable internal contract so later milestones (agenda extensions, search, backlinks) build on top of it rather than around it.

Not in v0.1 Alpha: Time-tracking/clocking dashboard, capture inbox, full-text search, backlinks, graph view, and the interactive Merge Dialog — those land in later milestones (see [Roadmap](#roadmap)).

## How to contribute

Orgsidian is built spec-first: every change traces back to the PRD, the architecture's Logical Decisions, and an epic/story in [`_bmad-output/planning-artifacts/epics.md`](./_bmad-output/planning-artifacts/epics.md).

1. Read [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the local dev setup, Conventional Commits convention, fixture-placement rules, and testing strategy.
2. Browse open work by [`epic:N`](https://github.com/orgsidian/orgsidian/labels) or [`milestone:vX.X`](https://github.com/orgsidian/orgsidian/issues) labels on [Issues](https://github.com/orgsidian/orgsidian/issues), or the [Roadmap Project board](https://github.com/orgs/orgsidian/projects/1).
3. Bug reports and feature discussion are welcome as GitHub Issues. Security vulnerabilities should instead follow [`SECURITY.md`](./SECURITY.md) (GitHub Security Advisories, not a public issue).
4. For v0.1 Alpha, the current lead maintainer reviews and merges all PRs; expect the review/merge cadence to loosen as more contributors show up.

## Planning Artifacts

This project is being built following the [BMad Method](https://github.com/bmad-code-org/BMAD-METHOD). Planning artifacts live under [`_bmad-output/`](./_bmad-output/):

- **PRD** — [`planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md`](./_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md)
- **Architecture** — [`planning-artifacts/architecture.md`](./_bmad-output/planning-artifacts/architecture.md) (55 numbered Logical Decisions)
- **Epics & stories** — [`planning-artifacts/epics.md`](./_bmad-output/planning-artifacts/epics.md) (13 epics, 108 stories across v0.1 → v0.5 → v1.0)
- **Test design** — [`test-artifacts/test-design.md`](./_bmad-output/test-artifacts/test-design.md) (authoritative system-level test strategy)

## Roadmap

| Milestone | Scope | Issues |
|---|---|---|
| **v0.1 Alpha** | Foundation, Parser round-trip, Vault+Index, Editor, External-edit fallback, First launch + Day-one Agenda | [`milestone:v0.1`](https://github.com/orgsidian/orgsidian/issues?q=label%3A%22milestone%3Av0.1%22) (61 stories) |
| **v0.5 Beta** | Today Dashboard + Clock, Capture/Search/Backlinks, Full Merge Dialog, Report Export, Customization & Plugin lock | [`milestone:v0.5`](https://github.com/orgsidian/orgsidian/issues?q=label%3A%22milestone%3Av0.5%22) (40 stories) |
| **v1.0** | Windows parity, Auto-update, Interactive Tutorial, Clock UX polish, a11y, full docs | [`milestone:v1.0`](https://github.com/orgsidian/orgsidian/issues?q=label%3A%22milestone%3Av1.0%22) (7 stories) |

Browse by epic with the [`epic:N`](https://github.com/orgsidian/orgsidian/labels) labels.

## Development

```sh
git clone https://github.com/orgsidian/orgsidian.git
cd orgsidian
pnpm install                          # commitlint + husky + shell-ui deps
cargo build --workspace --locked
pnpm tauri dev                        # launches the Tauri window
```

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the full toolchain prerequisites, the CI-parity one-liner, and the Conventional Commits convention (enforced locally by a `husky` `commit-msg` hook running `commitlint`).

## License

MIT (per architecture LD-1) — see [`LICENSE`](./LICENSE).

---

*Orgsidian is the integrated planner-and-knowledge desktop app for people who want org-mode without Emacs.*
