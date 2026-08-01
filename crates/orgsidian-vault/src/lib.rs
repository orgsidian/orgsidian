//! orgsidian-vault: vault designation + atomic write subsystem + dirty-buffer manager (FR-3, FR-4, FR-5).
//!
//! Story 3.1 ships the production atomic-write subsystem: [`atomic_write`]
//! wraps the `atomic-write-file` crate (LD-8) in a 3-attempt AV-aware
//! exponential-backoff loop and returns path-contextualized [`VaultError`]s
//! (the `io::Result<()>` surface from Story 1.9 is upgraded per the Epic 3
//! AC — `.expect(...)` call sites keep compiling since `VaultError: Debug`).
//! [`clean_orphan_temp_files`] collects temp residue from dead writers; Story
//! 3.6 wires it into the Vault-open flow.
//!
//! Story 3.2 ships the Dirty Buffer registry alongside: [`DirtyBufferManager`]
//! tracks which open files hold unsaved edits (LD-7 Single Writer Rule), the
//! state Epic 5 will consult to route an external write to auto-reload or the
//! Merge Dialog (FR-16). It is a pure in-memory type — shared via
//! [`SharedDirtyBuffers`], never touching the filesystem itself.

pub mod atomic;
pub mod dirty_buffer;
pub mod error;

pub use atomic::{atomic_write, clean_orphan_temp_files, CleanupReport};
pub use dirty_buffer::{DirtyBufferManager, SharedDirtyBuffers};
pub use error::VaultError;
