> Auto-synced from `_bmad-output/planning-artifacts/epics.md` by `tools/issues-sync`. Manual edits below this line will be **overwritten** on next sync; status label drift is preserved.

**Epic:** 1 &middot; **Milestone:** v0.1

---


As the **author / contributor**,
I want a working `pnpm create tauri-app@2` scaffold with React + TypeScript + identifier `com.orgsidian.app`,
So that `pnpm tauri dev` launches a Tauri window on macOS-arm64 and Ubuntu-LTS, ready for incremental refactor.

**Acceptance Criteria:**

**Given** an empty project root,
**When** `pnpm create tauri-app@2` is run with project name `orgsidian`, identifier `com.orgsidian.app`, React + TS, pnpm,
**Then** the resulting scaffold builds via `pnpm tauri build` on macOS-arm64 and Ubuntu-LTS
**And** `pnpm tauri dev` opens a Tauri window with the default React scaffold content
**And** root `LICENSE` (MIT) and `README.md` exist with project name and one-paragraph description.


---

**Source:** [`_bmad-output/planning-artifacts/epics.md` line 425](https://github.com/orgsidian/orgsidian/blob/main/_bmad-output/planning-artifacts/epics.md#L425)
