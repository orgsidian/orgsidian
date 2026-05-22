//! Host-emitted event vocabulary observed by plugins via
//! [`crate::OrgsidianPlugin::on_event`].

use crate::payload::{AgendaQuery, CaptureEntry};

/// Events the host emits during normal operation.
///
/// Variant naming follows the architecture **Naming Conventions** LD:
/// `PascalCase`, past-tense for completion events. `#[non_exhaustive]` is the
/// forward-compatibility hedge — consumers MUST include a wildcard `_` arm
/// when matching, so adding a new variant lands as a SemVer-minor bump per
/// LD-26.
///
/// `Clone` is required because [`crate::HookContext::emit_event`] may need
/// to clone for fan-out across multiple observers; `Debug` is required for
/// structured panic-logging inside the host-side plugin-hook dispatch
/// machinery landing in Story 1.8.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Event {
    /// A file in the active Vault has been opened in the editor. Emitted by
    /// the editor surface (Stories 4.x).
    FileOpened {
        /// Vault-relative path of the file just opened.
        path: String,
    },
    /// A file in the active Vault has been persisted to disk. Emitted by
    /// the atomic-write subsystem (Story 3.1) after `fsync` completes.
    FileSaved {
        /// Vault-relative path of the file just saved.
        path: String,
    },
    /// A file in the active Vault changed on disk via an external editor.
    /// Emitted by the filesystem watcher (Story 5.1) after debounce.
    FileChanged {
        /// Vault-relative path of the file whose external change was
        /// observed.
        path: String,
    },
    /// A headline inside a Vault file was edited in the Orgsidian editor.
    /// Emitted by the semantic layer (Story 2.3) when an edit crosses a
    /// headline boundary.
    HeadlineEdited {
        /// Vault-relative path of the file containing the headline.
        file: String,
        /// Stable headline identifier (semantics defined host-side).
        headline_id: String,
    },
    /// A clock entry was started against a headline. Emitted by the clock
    /// manager (Story 7.6).
    ClockStarted {
        /// Vault-relative path of the file containing the headline.
        file: String,
        /// Stable headline identifier (semantics defined host-side).
        headline_id: String,
    },
    /// A clock entry was stopped against a headline. Emitted by the clock
    /// manager (Story 7.6).
    ClockStopped {
        /// Vault-relative path of the file containing the headline.
        file: String,
        /// Stable headline identifier (semantics defined host-side).
        headline_id: String,
    },
    /// A Quick Capture entry was committed to the inbox. Emitted by the
    /// capture subsystem (Story 8.1) after the entry has been appended.
    CaptureSubmitted {
        /// The committed capture entry. Payload shape grows additively per
        /// LD-26 (see [`crate::CaptureEntry`]).
        entry: CaptureEntry,
    },
    /// An agenda query has been executed. Emitted by the agenda subsystem
    /// (Stories 6.3 / 6.4 / 7.x) once results are ready.
    AgendaQueried {
        /// The query that was executed. Payload shape grows additively per
        /// LD-26 (see [`crate::AgendaQuery`]).
        query: AgendaQuery,
    },
    /// The index has finished rebuilding. Unit-like by design: consumers
    /// that need rebuild details query the index after. Adding a payload
    /// here later (e.g., `{ files_indexed: u64 }`) is a SemVer-minor bump
    /// the `#[non_exhaustive]` enum-attribute permits.
    IndexRebuilt,
}
