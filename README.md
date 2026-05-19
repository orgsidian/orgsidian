# Orgsidian

[![Repo](https://img.shields.io/badge/repo-orgsidian%2Forgsidian-181717?logo=github)](https://github.com/orgsidian/orgsidian) ![Status](https://img.shields.io/badge/v0.1%20Alpha-in%20progress-yellow) ![License](https://img.shields.io/badge/license-MIT-blue)

> The cross-platform desktop **planner-and-knowledge** app that opens, edits, and organizes `.org` files (Emacs org-mode format) — without requiring Emacs. Local-first, OSS, format-faithful.

**Status: v0.1 Alpha — in progress (pre-Alpha development).** Repository is private during pre-Alpha (Months 1 → v0.1 Alpha release tag) and will flip to public at the v0.1 announcement (per architecture LD-5).

---

## Why

Org-mode is the most powerful integrated planner-and-knowledge format ever shipped, but it lives inside Emacs — and Emacs is a hard barrier. Orgsidian is *desktop-native, cross-platform, OSS, and faithful to the format*, treating tasks, time, and notes as peers on a unified surface.

The marketing wedge is **task-first** — "planner powered by org-mode" — but the product underneath is the integration itself.

## Planning Artifacts

This project is being built solo following the [BMad Method](https://github.com/bmad-code-org/BMAD-METHOD). Planning artifacts live under [`_bmad-output/`](./_bmad-output/):

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
# Install JS tooling (commitlint + husky)
pnpm install

# Sync epics.md → GitHub Issues (idempotent)
./scripts/sync-epics-to-github.sh
```

All commits must follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) — enforced locally by `husky` `commit-msg` hook running `commitlint`. The full enforcement chain (CI gate + `git-cliff` for CHANGELOG generation) lands in Stories 1.14 / 1.15.

## License

MIT (per architecture LD-1).

---

*Orgsidian is the integrated planner-and-knowledge desktop app for people who want org-mode without Emacs.*
