<!-- Implements FR-5 — Emacs keybindings mode reference & gap register (Story 4.7). -->

# Emacs keybindings mode

Orgsidian ships **cross-platform native keybindings** by default (Cmd on macOS,
Ctrl on Linux/Windows). For users with Emacs / org-mode muscle memory there is an
**opt-in Emacs keybindings mode** that rebinds the editor actions to their
familiar Emacs chords (`C-x C-s` to save, `C-c C-t` to cycle a TODO, and so on).

Native is the default and Emacs is **strictly opt-in** — consistent with
Orgsidian's lighthouse persona (CLI-confident but not an Emacs user). Enabling
Emacs mode never hides the native reference; both chord sets are always shown in
**Settings → Keybindings** so either idiom is discoverable.

## Enabling it

Open **Settings → Keybindings** and turn on **Emacs keybindings mode**. The
change takes effect immediately for the whole app and is preserved without
reloading the open file (your cursor and any unsaved edits are untouched).

When Emacs mode is active, the Emacs chord set **takes precedence** over the
native defaults: only one set is wired into the editor at a time, so there is no
ambiguity — the active keymap always wins on any conflict.

> **Session-scoped by design.** Per Orgsidian's "defaults are absolute at
> cold-start" rule (UX spec Principle 3), the keymap choice is a *session*
> preference: every cold start lands on the native default. Recalling the choice
> across restarts belongs to the future **Reopen last session** Settings opt-in.

## The Emacs chord set

Chords follow real Emacs / org-mode bindings, so your fingers carry over.
Multi-stroke chords (e.g. `C-x C-s`) are typed as a sequence: press the first
stroke, then the second. `C-` is the literal **Ctrl** key on *every* platform
(it is never remapped to Cmd on macOS), and `M-` is **Meta** (Alt / Option).

| Action            | Emacs chord   | Emacs command          | Status |
| ----------------- | ------------- | ---------------------- | ------ |
| Cycle TODO state  | `C-c C-t`     | `org-todo`             | Live |
| Toggle checkbox   | `C-c C-c`     | `org-ctrl-c-ctrl-c`    | Live |
| Set Schedule      | `C-c C-s`     | `org-schedule`         | Live |
| Set Deadline      | `C-c C-d`     | `org-deadline`         | Live |
| Save              | `C-x C-s`     | `save-buffer`          | Reserved |
| Open file         | `C-x C-f`     | `find-file`            | Reserved |
| Capture           | `C-c c`       | `org-capture`          | Reserved |
| Open Agenda       | `C-c a`       | `org-agenda`           | Reserved |
| Clock in          | `C-c C-x C-i` | `org-clock-in`         | Reserved |
| Clock out         | `C-c C-x C-o` | `org-clock-out`        | Reserved |

**Live** actions work today. **Reserved** actions have their Emacs chord
documented and wired to a "coming soon" placeholder — the underlying feature
(disk write-back, quick-open, Capture, Agenda, time tracking) ships in a later
epic, exactly as in the native default keymap. No reserved chord performs a fake
action; the chord is simply held so the map is complete and stable.

## Gaps

Story 4.7 covers the editor-owned org actions. The following gaps are known and
tracked; each is either an Emacs chord with no live Orgsidian action yet, or an
Orgsidian action with no idiomatic Emacs binding.

### Emacs chords whose action is not yet implemented (reserved)

`C-x C-s` (save), `C-x C-f` (open), `C-c c` (capture), `C-c a` (agenda),
`C-c C-x C-i` / `C-c C-x C-o` (clock in / out) are documented and reserved. They
surface a "coming soon" affordance until their features land in later epics
(disk write-back, Epic 8 Search/Capture, Epic 7 Agenda & time tracking). This
matches the native default keymap, where the same actions are reserved.

### Orgsidian actions with no idiomatic Emacs binding

- **Find / Replace.** Emacs mode does **not** remap search. Find stays on the
  native chord (`Cmd/Ctrl+F`), because find is owned by CodeMirror's search
  keymap, which Emacs mode does not reconfigure. The idiomatic Emacs `C-s`
  (`isearch-forward`) / `C-r` (`isearch-backward`) incremental search is **not
  yet bound** — a future shortcut-registry story (OQ-UX-4 / LD-56) is the place
  to add a real incremental-search binding.
- **Switch editor mode** (Raw → Pseudo-WYSIWYG → Split). This is an Orgsidian
  affordance with **no org-mode analog**, so it has no Emacs chord. It keeps its
  native chord (`Cmd/Ctrl+Alt+M`) in Emacs mode, handled by the global mode
  switcher, which Emacs mode does not reconfigure.

### AC-example reconciliation

The Story 4.7 acceptance criteria illustrate the style with "`C-x C-s` save,
`C-c C-c` cycle TODO". `C-x C-s` (`save-buffer`) is adopted verbatim. The
"`C-c C-c` cycle TODO" example is reconciled to the **faithful org-mode**
bindings, guided by the project's Fidelity Lighthouse ("how would org-mode do
it?"):

- **`C-c C-t`** (`org-todo`) cycles the TODO state — the dedicated org-mode
  binding an Emacs org user actually reaches for.
- **`C-c C-c`** (`org-ctrl-c-ctrl-c`) is org-mode's context action; on a checkbox
  list item it **toggles the checkbox**, which is where it is bound here.

So `C-c C-c` and `C-c C-t` are both present and both faithful — the AC's single
illustrative example is split across the two real org bindings rather than
collapsing checkbox and TODO onto one chord.

## Prefix chords used by org-mode but not (yet) by Orgsidian

Real org-mode packs dozens of commands under the `C-c` and `C-c C-x` prefixes
(tags, priorities, refile, archive, sparse trees, and more). Orgsidian binds
only the subset above today; the remaining continuations of those prefixes are
unbound. As more org features land, their Emacs continuations will be added to
`shell-ui/src/components/editor/keybindings/emacs.ts` alongside the native ones.
