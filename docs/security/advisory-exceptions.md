# Advisory Exceptions Ledger (LD-37)

> Human-readable companion to [`deny.toml`](../../deny.toml) at the repo root.
> Establishes the LD-37 quarterly review discipline for accepted RustSec /
> pnpm advisories and per-crate license exceptions.

## How this works

LD-37 ([architecture.md §LD-37](../../_bmad-output/planning-artifacts/architecture.md))
defines the supply-chain hygiene floor for Orgsidian:

- `cargo audit` runs at default strictness — any RustSec advisory in the
  graph fails the check.
- `cargo deny check licenses` enforces a closed allowlist (`MIT`,
  `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unlicense`, `Zlib`,
  `MPL-2.0`). Anything else fails by construction.
- `pnpm audit --audit-level=moderate --prod` runs the JS-side parallel:
  medium-or-above advisories on prod deps fail the check.
- `pnpm run audit:licenses:js` (via [`scripts/check-pnpm-licenses.mjs`](../../scripts/check-pnpm-licenses.mjs))
  runs the same allowlist on prod npm deps.

When an advisory or license rejection is deliberately accepted (e.g., a low-
severity RustSec ID with no upstream patch, a transitive dep with a
non-default-allowlisted but tolerable SPDX expression), it goes into BOTH
the machine-readable enforcement file (`deny.toml` `[advisories].ignore` or
`[licenses].exceptions`) AND this ledger. Every row carries a rationale and
a **next review date 90 days out** — quarterly cadence.

Drift between `deny.toml` and this ledger is a process bug: either the
exception was added without the ledger row (silent acceptance) or the ledger
row was added without the enforcement entry (no actual effect). Story 1.8's
CI gate will only check the enforcement file; reviewers check both.

The cargo-side `[advisories].ignore` is the *durable* exception path.
Per-advisory CLI flags (`cargo audit --ignore RUSTSEC-XXXX-YYYY`) are
reserved for short-lived CI overrides — they don't carry the rationale, so
they don't satisfy LD-37.

## Active exceptions

### Cargo advisories (`deny.toml` `[advisories].ignore`)

| RUSTSEC ID         | Crate                        | Decision | Rationale                                                                                                                                                                                                                                                                                            | First accepted | Next review |
| ------------------ | ---------------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | ----------- |
| RUSTSEC-2024-0429 | `glib@0.18.5`                | accept   | Transitive via Tauri 2.x Linux gtk/glib stack (gtk → atk/cairo-rs/gdk → glib). Upstream fix needs glib >=0.20, not yet vendored by Tauri. Linux-only impact. Re-evaluate next Tauri bump.                                                                                                              | 2026-05-22     | 2026-08-20  |
| RUSTSEC-2024-0411 | `gdkwayland-sys`             | accept   | gtk-rs GTK3 bindings unmaintained (upstream gtk-rs no longer maintains gtk3). Transitive via Tauri 2.x Linux backend. cargo-audit `--deny warnings` would otherwise fail CI. Re-evaluate when Tauri 2.x migrates to gtk4. (Story 1.8 surface expansion.)                                              | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0412 | `gdk`                        | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0413 | `atk`                        | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0414 | `gdkx11-sys`                 | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0415 | `gtk`                        | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0416 | `atk-sys`                    | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0417 | `gdkx11`                     | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0418 | `gdk-sys`                    | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0419 | `gtk3-macros`                | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0420 | `gtk-sys`                    | accept   | gtk-rs GTK3 bindings unmaintained — same Tauri 2.x Linux backend transitive root.                                                                                                                                                                                                                    | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0436 | `paste`                      | accept   | `paste` macro crate unmaintained (upstream author archived). Transitive via the Tauri 2.x / specta proc-macro chain. No CVE; informational. Re-evaluate if a maintained fork emerges.                                                                                                                | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2024-0370 | `proc-macro-error`           | accept   | `proc-macro-error` v1.x unmaintained; transitive via the older proc-macro chain. No CVE. Re-evaluate at next major proc-macro family bump in the dep tree.                                                                                                                                           | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2025-0075 | `unic-char-range`            | accept   | `unic-*` crate family unmaintained (foundational Unicode tables) — transitive via specta/specta-macros. No CVE. Re-evaluate when specta migrates off the `unic-*` family (tracked as `icu` adoption).                                                                                                | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2025-0080 | `unic-common`                | accept   | `unic-*` crate family unmaintained — same root cause as RUSTSEC-2025-0075.                                                                                                                                                                                                                            | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2025-0081 | `unic-char-property`         | accept   | `unic-*` crate family unmaintained — same root cause as RUSTSEC-2025-0075.                                                                                                                                                                                                                            | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2025-0098 | `unic-ucd-version`           | accept   | `unic-*` crate family unmaintained — same root cause as RUSTSEC-2025-0075.                                                                                                                                                                                                                            | 2026-05-23     | 2026-08-21  |
| RUSTSEC-2025-0100 | `unic-ucd-ident`             | accept   | `unic-*` crate family unmaintained — same root cause as RUSTSEC-2025-0075.                                                                                                                                                                                                                            | 2026-05-23     | 2026-08-21  |

> **Note (Story 1.8):** RUSTSEC-2024-0411..0420, RUSTSEC-2024-0436, RUSTSEC-2024-0370, and RUSTSEC-2025-0075/-0080/-0081/-0098/-0100 are managed by passing `--ignore` flags to `cargo audit` in CI (see `.github/workflows/pr.yml` and `nightly.yml`). They are NOT (yet) duplicated into `deny.toml [advisories].ignore` because `deny.toml` already sets `unmaintained = "workspace"`, which makes `cargo deny check advisories` ignore transitive unmaintained warnings by construction. Future hardening (Story 1.10+ or next Tauri bump) may consolidate into `deny.toml`.

### Pnpm advisories (`docs/security/advisory-exceptions.md` ledger only)

| pnpm-audit ID | Package | Decision | Rationale | First accepted | Next review |
| ------------- | ------- | -------- | --------- | -------------- | ----------- |
| _(none)_      |         |          |           |                |             |

### Cargo duplicate-version skips (`deny.toml` `[bans].skip`)

All entries below are transitive-forced by the Tauri 2.11.x dep tree
(gtk/glib/wry/specta/tauri-plugin chain). None of them are the LD-37
canonical-version invariants (`tokio`, `serde`, `chrono`, `rusqlite`) —
the binding rule from AC2 holds. Re-evaluate at each Tauri-ecosystem bump.

| Crate(s)                                                                                                                                | Decision | First accepted | Next review |
| --------------------------------------------------------------------------------------------------------------------------------------- | -------- | -------------- | ----------- |
| `base64@0.21.7` (swift-rs)                                                                                                              | accept   | 2026-05-22     | 2026-08-20  |
| `bitflags@1.3.2` (legacy gtk-rs)                                                                                                        | accept   | 2026-05-22     | 2026-08-20  |
| `foldhash@0.1.5` (hashbrown 0.15)                                                                                                       | accept   | 2026-05-22     | 2026-08-20  |
| `getrandom@0.2.17`, `getrandom@0.3.4` (ring/rustls, rand_core)                                                                          | accept   | 2026-05-22     | 2026-08-20  |
| `hashbrown@0.12.3`, `hashbrown@0.15.5` (indexmap 1.x, schemars 0.8)                                                                     | accept   | 2026-05-22     | 2026-08-20  |
| `heck@0.4.1` (older proc-macros)                                                                                                        | accept   | 2026-05-22     | 2026-08-20  |
| `indexmap@1.9.3` (schemars 0.8 + Tauri build chain)                                                                                     | accept   | 2026-05-22     | 2026-08-20  |
| `nix@0.30.1` (atomic-write-file 0.3; tauri-plugin-os pins nix 0.31)                                                                     | accept   | 2026-05-23     | 2026-08-21  |
| `png@0.17.16` (ico → tauri-codegen)                                                                                                     | accept   | 2026-05-22     | 2026-08-20  |
| `proc-macro-crate@1.3.1`, `proc-macro-crate@2.0.2`                                                                                      | accept   | 2026-05-22     | 2026-08-20  |
| `serde_spanned@0.6.9` (toml 0.8)                                                                                                        | accept   | 2026-05-22     | 2026-08-20  |
| `syn@1.0.109` (legacy proc-macros)                                                                                                      | accept   | 2026-05-22     | 2026-08-20  |
| `thiserror@1.0.69`, `thiserror-impl@1.0.69`                                                                                             | accept   | 2026-05-22     | 2026-08-20  |
| `toml@0.8.2`, `toml@0.9.12+spec-1.1.0` (Tauri build chain)                                                                              | accept   | 2026-05-22     | 2026-08-20  |
| `toml_datetime@0.6.3`, `toml_datetime@0.7.5+spec-1.1.0`                                                                                 | accept   | 2026-05-22     | 2026-08-20  |
| `toml_edit@0.19.15`, `toml_edit@0.20.2`                                                                                                 | accept   | 2026-05-22     | 2026-08-20  |
| `windows-link@0.1.3`                                                                                                                    | accept   | 2026-05-22     | 2026-08-20  |
| `windows-sys@0.59.0`, `windows-sys@0.60.2` (tao/wry pin 0.59; newer crates use 0.60/0.61)                                               | accept   | 2026-05-22     | 2026-08-20  |
| `windows-targets@0.52.6`                                                                                                                | accept   | 2026-05-22     | 2026-08-20  |
| `windows_x86_64_gnu@0.52.6`, `windows_x86_64_msvc@0.52.6`                                                                               | accept   | 2026-05-22     | 2026-08-20  |
| `winnow@0.5.40`, `winnow@0.7.15` (toml_edit 0.19/0.20)                                                                                  | accept   | 2026-05-22     | 2026-08-20  |

### License exceptions (additions to the LD-37 allowlist)

The LD-37 allowlist in `deny.toml` `[licenses].allow` and the JS-side
allowlist in `scripts/check-pnpm-licenses.mjs` are kept in lockstep. All
additions below are OSI- and/or FSF-recognised permissive licenses; the
listed crates / packages forced the question by appearing transitively.

| SPDX                              | Pulled in by                                              | Rationale                                                                                       | First accepted | Next review |
| --------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | -------------- | ----------- |
| `Unicode-3.0`                     | `icu_*` / `idna` / `url` (foundational Unicode handling)  | Permissive Unicode license; widely adopted across the Rust ecosystem.                           | 2026-05-22     | 2026-08-20  |
| `BSL-1.0`                         | Boost Software License — small Rust crates (e.g. ryu)     | OSI/FSF approved permissive license, GPL-compatible.                                            | 2026-05-22     | 2026-08-20  |
| `Apache-2.0 WITH LLVM-exception`  | LLVM-derived crates                                       | Apache-2.0 with relaxed patent clause for LLVM compiler infra; permissive.                      | 2026-05-22     | 2026-08-20  |
| `0BSD` (pnpm-side only)           | `tslib`                                                   | Zero-clause BSD; OSI-approved, equivalent to public domain.                                     | 2026-05-22     | 2026-08-20  |
| `CC-BY-4.0` (pnpm-side only)      | `caniuse-lite`                                            | Creative Commons Attribution 4.0; appropriate for browser-data tables (data, not source code).  | 2026-05-22     | 2026-08-20  |

## Review history

| Date       | Reviewer | Notes                                                                 |
| ---------- | -------- | --------------------------------------------------------------------- |
| 2026-05-22 | Tiziano  | Story 1.7 establishes the ledger. Transitive-forced batch accepted at landing (1 advisory, 23 dup-version skips, 5 license additions). Tauri 2.11.x is the common upstream cause. Re-evaluate on next Tauri-ecosystem bump. |

## Cross-references

- LD-37 source: [`architecture.md#LD-37`](../../_bmad-output/planning-artifacts/architecture.md)
- Enforcement file: [`deny.toml`](../../deny.toml) at repo root
- Pnpm license filter: [`scripts/check-pnpm-licenses.mjs`](../../scripts/check-pnpm-licenses.mjs)
- Security policy (forward-reference, lands in Story 1.10): `SECURITY.md`
