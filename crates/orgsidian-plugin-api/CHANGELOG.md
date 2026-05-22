# Changelog

All notable changes to `orgsidian-plugin-api` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
internally from day 1 even though it is not published to crates.io until v1.5+
(see LD-10 / LD-33 in `_bmad-output/planning-artifacts/architecture.md`).

## [Unreleased]

## [0.0.0] - 2026-05-22

### Added

- Initial trait surface (Story 1.5):
  - `OrgsidianPlugin` trait with `metadata`, `init`, `shutdown`, `priority`, `on_event`, `on_save_before`, `on_capture_before`, `on_agenda_query_after` methods (LD-26).
  - `Event` enum (`#[non_exhaustive]`) with v1.0 variants: `FileOpened`, `FileSaved`, `FileChanged`, `HeadlineEdited`, `ClockStarted`, `ClockStopped`, `CaptureSubmitted`, `AgendaQueried`, `IndexRebuilt`.
  - `HookOutcome<T>` (`#[non_exhaustive]`) with `Continue`, `Replace(T)`, `Cancel(String)`.
  - `HookContext` and `PluginContext` traits (`Send + Sync`; passed to plugins as `&dyn` references per LD-5 round-4 amendment).
  - `PluginMetadata`, `CaptureEntry`, `AgendaQuery`, `AgendaItem` payload structs (day-1 minimal shapes; SemVer-additive growth path).
  - `PluginError` enum + `Result<T>` alias (local to leaf; separate from `orgsidian-core::OrgError`).
