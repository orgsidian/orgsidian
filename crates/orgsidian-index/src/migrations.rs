//! Forward-only migration runner (LD-12 migrations, LD-13 rebuild-on-drift,
//! LD-11 schema, LD-14 connection management, FR-17 derived index).
//!
//! Wires [`rusqlite_migration`] onto the version-1 DDL ([`crate::SCHEMA_SQL`],
//! which is `migrations/0001_initial-schema.sql`). There is exactly one
//! migration — `0001` — because the index is forward-only: it is a derived
//! artifact rebuildable from the Vault's `.org` files (LD-13, LD-17), so there
//! is no `down`/rollback path. Schema version 2 arrives as `0002_*.sql`, never
//! as an edit to `0001`.
//!
//! [`migrate`] is the versioned production path; [`crate::apply_schema`] is a
//! DDL-only primitive kept for schema-shape tests (see the two functions' docs).

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::IndexError;
use crate::SCHEMA_SQL;

/// A short human label recorded in the `_schema_version` audit row for v1.
const V1_DESCRIPTION: &str = "initial schema (Story 3.3 baseline)";

/// Build the migration set — a factory, not a `static`.
///
/// Returning a fresh `Migrations` on each call keeps the crate free of a
/// `once_cell`/`LazyLock` dependency (and the MSRV question one would raise):
/// `SCHEMA_SQL` is `&'static str`, so the set is `Migrations<'static>`, and
/// constructing a one-element `Vec` per call is trivially cheap next to the
/// single `to_latest` a database open triggers. Both [`migrate`] and the tests
/// call this one builder, so the migration set has a single definition and
/// cannot fork.
///
/// `M::up_with_hook` (not bare `M::up`) splits the migration cleanly: the SQL
/// half is `SCHEMA_SQL`, kept **pure DDL** so it stays identical to what
/// [`crate::apply_schema`] runs; the Rust hook writes the `_schema_version`
/// audit row. `rusqlite_migration` runs each migration in its own transaction
/// and executes the hook *after* the SQL succeeds within that same transaction,
/// so `SCHEMA_SQL`'s deliberate absence of `BEGIN;`/`COMMIT;` (the Story 3.3
/// seam) is exactly what makes this compose — an embedded `BEGIN` would fail
/// with "cannot start a transaction within a transaction".
fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up_with_hook(
        SCHEMA_SQL,
        |tx: &rusqlite::Transaction| {
            // `version` is bound EXPLICITLY to 1, not left to auto-assign: the
            // column is `INTEGER PRIMARY KEY`, i.e. a rowid alias, so an unbound
            // INSERT would silently invent a plausible version. Binding it is what
            // makes `_schema_version.version` AGREE with `PRAGMA user_version`
            // (which rusqlite_migration sets to 1 for one applied migration) rather
            // than coincide by luck. `applied_at` is ISO-8601 TEXT from SQLite's own
            // `strftime` — no `chrono` edge, deterministic in shape.
            tx.execute(
                "INSERT INTO _schema_version (version, description, applied_at)
             VALUES (1, ?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                [V1_DESCRIPTION],
            )?;
            Ok(())
        },
    )])
}

/// Apply all forward-only migrations, bringing `conn` to the latest schema
/// version (LD-12).
///
/// This is the **versioned production path**: it bumps `PRAGMA user_version`
/// (the value LD-13's drift check compares) and writes the `_schema_version`
/// audit row. It **supersedes** [`crate::apply_schema`] for real use —
/// `apply_schema` remains a DDL-only primitive for schema-shape tests and does
/// neither of those things.
///
/// It is **idempotent**: calling it on an already-current database is a no-op
/// (`Ok`), leaving `user_version` and the audit row untouched — the property
/// `apply_schema` deliberately lacks (re-running the bare DDL fails loudly).
///
/// Takes `&mut Connection` because `to_latest` needs exclusive access to run
/// each migration in its own transaction. It is deliberately **not** folded
/// into [`crate::open`]: in the Story 3.5 connection model the single writer
/// migrates once at startup and the reader pool must never run migrations, so
/// coupling migration onto every open would push it onto the read path.
///
/// # Errors
///
/// [`IndexError::Migration`] if a migration's SQL or its hook fails.
pub fn migrate(conn: &mut Connection) -> Result<(), IndexError> {
    migrations().to_latest(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The migration set is well-formed: `validate` runs the up-migration on a
    /// throwaway in-memory database, catching a malformed `0001` here at test
    /// time rather than on a user's first real open. (Its internal `:memory:`
    /// is the crate's own harness, not our on-disk fixture — the integration
    /// tests in `tests/migrations.rs` drive a real `.db`.)
    #[test]
    fn the_migration_set_validates() {
        migrations()
            .validate()
            .expect("migration 0001 is well-formed");
    }
}
