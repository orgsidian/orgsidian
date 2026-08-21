//! Clean-buffer reload model: the pure cursor-preservation decision + the
//! buffer-refresh plan an external write onto a CLEAN buffer produces (FR-16
//! auto-reload; LD-7 Single Writer Rule / LD-9 external-edits).
//!
//! FR-16 splits an external write two ways by Dirty-Buffer state
//! ([`crate::dirty_buffer::DirtyBufferManager::is_dirty`]): a DIRTY buffer takes
//! the conflict path ([`crate::conflict`]), a CLEAN buffer auto-reloads from
//! disk. This module owns the CLEAN half's pure data model — what the new buffer
//! content is and where the cursor lands — with **no I/O**: the caller (the
//! `orgsidian-core` reconciler, Story 5.4) reads the disk content and drives the
//! index re-sync; here we only decide.
//!
//! # The cursor rule
//!
//! "Preserve the cursor if its source line is unchanged; otherwise reset to
//! top" (FR-16). [`decide_cursor`] compares the line at the cursor's zero-based
//! line index in the OLD buffer and the NEW disk content: identical text →
//! [`CursorOutcome::Preserved`] (column clamped to the line's char length);
//! anything else — the line's text changed, or that line index no longer exists
//! because lines were deleted above it — → [`CursorOutcome::ResetToTop`]. This
//! is a same-index comparison, not a content-following heuristic: a line
//! inserted *above* the cursor shifts the original text to a new index and reads
//! as "changed" (reset), which is the deterministic, testable reading of the AC.
//!
//! # Purity
//!
//! Every type here is pure and in-memory: no filesystem, no `Result`, no
//! `tracing` — mirroring [`crate::conflict`] and [`crate::dirty_buffer`]. The
//! new buffer content is the user's notes, so [`BufferReload`] carries a
//! **redacting `Debug`** (content byte-length, never the bytes) exactly as those
//! modules do.

use std::fmt;

/// A zero-based editor cursor position: `line` counts lines from 0, `column`
/// counts characters (not bytes) from the line start.
///
/// Chars, not bytes, so the column survives the byte↔char mismatch of multibyte
/// UTF-8 note content; [`decide_cursor`] clamps it to the preserved line's char
/// length.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CursorPosition {
    /// Zero-based line index.
    pub line: usize,
    /// Zero-based column, counted in characters.
    pub column: usize,
}

impl CursorPosition {
    /// The top-of-document position `(0, 0)` — where a reset cursor lands.
    pub const TOP: CursorPosition = CursorPosition { line: 0, column: 0 };

    /// Construct a position from a `(line, column)` pair.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Where the cursor lands after a clean-buffer reload (FR-16).
///
/// `#[non_exhaustive]` matching the crate's forward-compatible enum convention:
/// a future outcome (e.g. a content-following "moved to" position) can be added
/// without breaking downstream `match`es.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorOutcome {
    /// The cursor's source line was unchanged — keep the position (column
    /// clamped to the new line's char length).
    Preserved(CursorPosition),
    /// The source line changed or no longer exists — reset to the top of the
    /// document (`(0, 0)`).
    ResetToTop,
}

impl CursorOutcome {
    /// The concrete position this outcome resolves to — the preserved position,
    /// or [`CursorPosition::TOP`] for a reset. Lets a caller apply the cursor
    /// without re-matching.
    #[must_use]
    pub fn resolved(&self) -> CursorPosition {
        match self {
            CursorOutcome::Preserved(position) => *position,
            CursorOutcome::ResetToTop => CursorPosition::TOP,
        }
    }

    /// Whether the cursor position was preserved (vs reset to top).
    #[must_use]
    pub fn is_preserved(&self) -> bool {
        matches!(self, CursorOutcome::Preserved(_))
    }
}

/// Decide where the cursor lands when a CLEAN buffer is reloaded from disk.
///
/// Preserves `cursor` iff the line at `cursor.line` has identical text in
/// `old_content` (the buffer before the external write) and `new_content` (the
/// fresh disk content); otherwise resets to top. When preserved, the column is
/// clamped to the new line's char length so a shortened line never leaves the
/// cursor past the end. Lines are taken with [`str::lines`] (no synthetic
/// trailing-empty element) for both sides, so the comparison is symmetric.
#[must_use]
pub fn decide_cursor(
    old_content: &str,
    new_content: &str,
    cursor: CursorPosition,
) -> CursorOutcome {
    let old_line = old_content.lines().nth(cursor.line);
    let new_line = new_content.lines().nth(cursor.line);
    match (old_line, new_line) {
        (Some(old), Some(new)) if old == new => {
            let line_len = new.chars().count();
            CursorOutcome::Preserved(CursorPosition {
                line: cursor.line,
                column: cursor.column.min(line_len),
            })
        }
        _ => CursorOutcome::ResetToTop,
    }
}

/// The plan for refreshing a CLEAN buffer from disk: the new content plus the
/// cursor decision. Pure — the caller performs the actual buffer swap.
///
/// Fields are private (the content is user notes); read through the getters.
pub struct BufferReload {
    new_content: String,
    cursor: CursorOutcome,
}

/// Redacting `Debug`: prints the cursor outcome and the new content's
/// *byte-length*, never the content itself — the same privacy guarantee
/// [`crate::conflict::ConflictState`] and
/// [`crate::dirty_buffer::DirtyBufferManager`] hold (the reloaded content is the
/// user's notes and must never reach a log/panic/`{:?}`).
impl fmt::Debug for BufferReload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferReload")
            .field("new_content_len", &self.new_content.len())
            .field("cursor", &self.cursor)
            .finish()
    }
}

impl BufferReload {
    /// Build the reload plan from the old buffer, the fresh disk content, and
    /// the current cursor. Takes ownership of `disk_content` as the new buffer
    /// and runs [`decide_cursor`] against `old_content`.
    #[must_use]
    pub fn plan(
        old_content: &str,
        disk_content: impl Into<String>,
        cursor: CursorPosition,
    ) -> Self {
        let new_content = disk_content.into();
        let cursor = decide_cursor(old_content, &new_content, cursor);
        Self {
            new_content,
            cursor,
        }
    }

    /// The refreshed buffer content (the on-disk bytes the caller reloads).
    #[must_use]
    pub fn new_content(&self) -> &str {
        &self.new_content
    }

    /// Consume the plan, yielding the owned new content (avoids a clone at the
    /// call site that hands the content on to the frontend).
    #[must_use]
    pub fn into_new_content(self) -> String {
        self.new_content
    }

    /// Where the cursor lands after the reload.
    #[must_use]
    pub fn cursor(&self) -> CursorOutcome {
        self.cursor
    }
}

/// Free-function alias for [`BufferReload::plan`], for callers that prefer a
/// `reload::plan(...)` spelling next to `decide_cursor`.
#[must_use]
pub fn plan(
    old_content: &str,
    disk_content: impl Into<String>,
    cursor: CursorPosition,
) -> BufferReload {
    BufferReload::plan(old_content, disk_content, cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "* Alpha\nshared body line\n* Beta v1\n";

    #[test]
    fn preserves_cursor_when_line_unchanged() {
        // "shared body line" is line index 1, identical in both.
        let new = "* Alpha\nshared body line\n* Beta v2 CHANGED\n";
        let outcome = decide_cursor(OLD, new, CursorPosition::new(1, 4));
        assert_eq!(outcome, CursorOutcome::Preserved(CursorPosition::new(1, 4)));
        assert!(outcome.is_preserved());
        assert_eq!(outcome.resolved(), CursorPosition::new(1, 4));
    }

    #[test]
    fn resets_when_line_text_changed() {
        // Line index 2 changed: "* Beta v1" → "* Beta v2 CHANGED".
        let new = "* Alpha\nshared body line\n* Beta v2 CHANGED\n";
        let outcome = decide_cursor(OLD, new, CursorPosition::new(2, 3));
        assert_eq!(outcome, CursorOutcome::ResetToTop);
        assert_eq!(outcome.resolved(), CursorPosition::TOP);
    }

    #[test]
    fn resets_when_line_deleted_out_of_range() {
        // New content has fewer lines; the cursor's line index no longer exists.
        let new = "* Alpha\n";
        let outcome = decide_cursor(OLD, new, CursorPosition::new(2, 0));
        assert_eq!(outcome, CursorOutcome::ResetToTop);
    }

    #[test]
    fn preserves_but_clamps_column_past_new_line_end() {
        // Same line index 0 kept identical, but the cursor column is well past
        // its char length (7 chars in "* Alpha").
        let new = "* Alpha\nreplaced\n";
        let outcome = decide_cursor(OLD, new, CursorPosition::new(0, 999));
        assert_eq!(outcome, CursorOutcome::Preserved(CursorPosition::new(0, 7)));
    }

    #[test]
    fn column_counts_chars_not_bytes_for_multibyte_lines() {
        // "café note" is 9 chars; a within-range column is kept verbatim.
        let old = "café note\nz\n";
        let new = "café note\nCHANGED\n";
        let outcome = decide_cursor(old, new, CursorPosition::new(0, 6));
        assert_eq!(outcome, CursorOutcome::Preserved(CursorPosition::new(0, 6)));
        // A past-end column clamps to the char count (9), not the byte count (10).
        let clamped = decide_cursor(old, new, CursorPosition::new(0, 50));
        assert_eq!(clamped, CursorOutcome::Preserved(CursorPosition::new(0, 9)));
    }

    #[test]
    fn identical_content_preserves_everywhere() {
        let outcome = decide_cursor(OLD, OLD, CursorPosition::new(2, 5));
        // Line "* Beta v1" is 9 chars; column 5 is within range → kept.
        assert_eq!(outcome, CursorOutcome::Preserved(CursorPosition::new(2, 5)));
    }

    #[test]
    fn plan_carries_new_content_and_cursor() {
        let new = "* Alpha\nshared body line\n* Beta v2 CHANGED\n";
        let reload = plan(OLD, new, CursorPosition::new(1, 2));
        assert_eq!(reload.new_content(), new);
        assert_eq!(
            reload.cursor(),
            CursorOutcome::Preserved(CursorPosition::new(1, 2))
        );
        assert_eq!(reload.into_new_content(), new);
    }

    /// `Debug` must never leak the reloaded note text; it must still show the
    /// cursor outcome and the content byte-length for debugging.
    #[test]
    fn debug_redacts_reloaded_content() {
        let reload = plan("old\n", "SECRET NOTE BODY", CursorPosition::TOP);
        let rendered = format!("{reload:?}");
        assert!(!rendered.contains("SECRET"), "no note text: {rendered}");
        assert!(
            rendered.contains("new_content_len: 16"),
            "byte length shown: {rendered}"
        );
        assert!(rendered.contains("ResetToTop"), "cursor shown: {rendered}");
    }
}
