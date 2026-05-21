# Deferred Work

## Deferred from: code review of story-1.1 (2026-05-19)

- **CSP disabled in `src-tauri/tauri.conf.json:22`** [MED] — `"csp": null` ships the app with no Content-Security-Policy; scaffold default. Harden before v0.1 Alpha (broad XSS surface once org-mode content rendering lands in Story 3+).
- **`vite.config.ts:5` `@ts-expect-error` will itself fail compilation if `@types/node` is later added** [MED] — directive becomes "unused" once the error it expects no longer occurs. Revisit when adding @types/node (Story 1.3 frontend wiring may force this).
- **`tsconfig.node.json:3` `composite: true` without `declaration`/`outDir`** [LOW] — `tsc -b` would write `.d.ts` next to `vite.config.ts`. Scaffold-emitted; revisit in Story 1.3 build-config retune.
- **`tsconfig.json` `references` block is dead under `tsc` (only honored by `tsc -b`)** [LOW] — `package.json:34` runs bare `tsc`, so `vite.config.ts` type-checking does NOT happen at build time. Story 1.3.
- **`src/App.tsx:11` `invoke('greet')` unhandled promise rejection** [LOW] — demo UI; Story 1.3 replaces.
- **`src/main.tsx:5` `document.getElementById("root") as HTMLElement` cast hides null** [LOW] — scaffold; Story 1.3.
- **`src/App.tsx:18-26,30` `<a target="_blank">` without `rel="noopener noreferrer"`** [LOW] — demo UI; Story 1.3.
- **`src/App.tsx:39-43` `<input>` half-controlled (no `value` binding)** [LOW] — demo UI; Story 1.3.
- **`src/App.tsx:39-43` `<input>` missing `<label>`/`aria-label`** [LOW] — demo UI accessibility; Story 1.3.
- **`.gitignore` ignores `src-tauri/gen/schemas/` but `src-tauri/capabilities/default.json:2` `$schema` references that path** [LOW] — editor JSON-schema warnings on fresh checkout until first `tauri dev`/`build`. Cosmetic; expected behavior.
- **`.gitignore` only ignores `src-tauri/gen/schemas/`, not future `gen/android/` or `gen/apple/`** [LOW] — mobile targets out of scope until post-v1.0; flag for when mobile is added.

## Deferred from: code review of story-1.2 (2026-05-21)

- **`crates/orgsidian-shell-app/Cargo.toml` shown as new file rather than rename** [LOW] — Content diverged enough during refactor that git rename detection fell below similarity threshold, breaking the blame trail on this single file. Other Tauri files (build.rs, icons, tauri.conf.json, src/lib.rs, src/main.rs) preserved rename history correctly. Cannot retroactively fix without rewriting history.
