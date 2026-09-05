//! The index façade — the only crate module that names both `orgsidian_parser`
//! and `orgsidian_index` (Story 3.6 Dev Note 1). Mirrors the `parser` façade
//! role: the leaves ship the mechanism (parse; SQLite + FTS sync), the hub
//! wires the domain (walk → parse → map → batch-submit; vault-open identity).
//!
//! - [`map`] does `Document` → `orgsidian_index::FileIndexInput`.
//! - [`scan`] owns [`scan_vault`] and the [`ScanProgress`]/[`ScanOutcome`] types.
//! - [`IndexHandle`] bundles the LD-14 writer + reader pool for a designated
//!   vault; [`designate_vault`] / [`open_index`] construct it (identity guard,
//!   fresh-stamp, `vault_meta`, `recent_vaults`).

mod map;
pub mod resync;
pub mod scan;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use orgsidian_index::{
    check_integrity, collect_stats, inspect_index_file, set_vault_meta, stamp_application_id,
    IndexError, IndexIdentity, IndexPool, IndexWriter,
};
use orgsidian_vault::VaultError;

use crate::error::OrgError;
use crate::settings;

pub use orgsidian_index::query::agenda::AgendaItem;
pub use orgsidian_index::{IndexStats, IntegrityCheck, IntegrityReport};
pub use resync::{resync_file, ResyncOutcome};
pub use scan::{scan_vault, ScanOutcome, ScanProgress};

/// The `vault_meta` key under which the canonical vault root is recorded.
const VAULT_ROOT_META_KEY: &str = "vault_root";

/// The OS-data-dir subdirectory the derived index databases live in (LD-40:
/// the index lives OUTSIDE the vault).
const INDEX_SUBDIR: &[&str] = &["orgsidian", "index"];

/// A live handle to a designated vault's derived index: the single LD-14 writer
/// task + the reader pool, plus the canonical vault root and the on-disk DB
/// path. The shell keeps this in managed state; the scan and (Epics 7/8) the
/// read API borrow it.
pub struct IndexHandle {
    writer: IndexWriter,
    pool: IndexPool,
    vault_root: PathBuf,
    db_path: PathBuf,
}

impl IndexHandle {
    /// The canonical vault root this index derives from.
    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    /// The on-disk path of the derived index database (outside the vault).
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// The LD-14 single writer task (write path).
    pub(crate) fn writer(&self) -> &IndexWriter {
        &self.writer
    }

    /// The LD-14 reader pool (incremental-skip reads; Epics 7/8 queries).
    pub(crate) fn pool(&self) -> &IndexPool {
        &self.pool
    }

    /// Shut the index down cleanly: drop the reader pool (closing its
    /// connections) and drain + close the writer task, so no connection
    /// outlives this handle. Call before deleting/replacing the index file or
    /// switching vaults (Epic 6) — otherwise a still-open connection can race a
    /// file deletion (a WAL `disk I/O error` on the next open).
    pub async fn shutdown(self) {
        let IndexHandle { writer, pool, .. } = self;
        drop(pool);
        writer.shutdown().await;
    }
}

/// Designate `vault_root` as the active vault: resolve the derived index DB
/// path in the OS data dir (LD-40), open/create + identity-guard the index, and
/// record the vault in `GlobalSettings.recent_vaults` (most-recent-first,
/// deduped, cap 10). Returns the [`IndexHandle`] the shell keeps and runs
/// [`scan_vault`] against.
///
/// Re-designating an unchanged, already-indexed vault does near-zero work (the
/// identity guard passes and every file's `(mtime, size)` matches → the
/// mechanism behind the epic's "cached fast-open").
///
/// # Errors
///
/// [`OrgError::Vault`] if the root cannot be resolved; [`OrgError::Index`] if
/// the DB is foreign or a writer/reader op fails; [`OrgError::Io`] if the OS
/// data dir is unavailable or the settings write fails.
pub async fn designate_vault(vault_root: &Path) -> Result<IndexHandle, OrgError> {
    let canonical = orgsidian_vault::canonicalize_vault_root(vault_root).map_err(vault_err)?;
    let db_path = default_index_db_path(&canonical)?;
    let handle = open_index(&canonical, &db_path).await?;
    push_recent_vault(&handle.vault_root)?;
    Ok(handle)
}

/// Open (or create) the derived index at `db_path` for `vault_root`, with the
/// LD-13 identity guard. The hermetic mechanism `designate_vault` wraps —
/// tests drive this directly with a `TempDir` DB path so they touch neither the
/// OS data dir nor `global.toml`.
///
/// Steps: canonicalize + orphan-temp cleanup (vault-open); classify a
/// pre-existing DB (`Foreign` → refuse; `VersionMismatch`/unreadable → drop +
/// rebuild; `Ours` → open cached); spawn the writer (migrates) + reader pool;
/// stamp `application_id` on a freshly created DB; record `vault_root` in
/// `vault_meta`.
///
/// # Errors
///
/// As [`designate_vault`], minus the `recent_vaults` write.
pub async fn open_index(vault_root: &Path, db_path: &Path) -> Result<IndexHandle, OrgError> {
    let canonical = orgsidian_vault::open_vault_root(vault_root).map_err(vault_err)?;

    // Classify a pre-existing file BEFORE spawning the writer, so a foreign
    // SQLite file is refused without `open()` converting its journal mode.
    let mut fresh = !db_path.exists();
    if db_path.exists() {
        match inspect_index_file(db_path) {
            Ok(IndexIdentity::Ours) => {}
            Ok(IndexIdentity::OursUnstamped) => {
                // First-time creation was interrupted after the migration but
                // before the `application_id` stamp (a crash between the two).
                // Re-stamp below — do NOT drop the file or refuse it.
                fresh = true;
            }
            Ok(IndexIdentity::VersionMismatch) => {
                // LD-13 drift → drop + rebuild (the rebuild is just a fresh
                // scan, free because the scan engine exists here).
                remove_index_files(db_path)?;
                fresh = true;
            }
            Ok(IndexIdentity::Foreign) => {
                return Err(OrgError::Index {
                    reason: format!(
                        "the index location {} holds a database that is not an Orgsidian index; \
                         refusing to overwrite it. Move or remove that file and try again.",
                        db_path.display()
                    ),
                });
            }
            Err(_) => {
                // Not a readable database (corrupt/partial write) → rebuild.
                remove_index_files(db_path)?;
                fresh = true;
            }
        }
    }

    let writer = IndexWriter::spawn(db_path).map_err(index_err)?;
    let pool = IndexPool::new(db_path).map_err(index_err)?;

    if fresh {
        writer
            .execute(|conn| stamp_application_id(conn))
            .await
            .map_err(index_err)?;
    }

    let root_value = canonical.to_string_lossy().into_owned();
    writer
        .execute(move |conn| set_vault_meta(conn, VAULT_ROOT_META_KEY, &root_value))
        .await
        .map_err(index_err)?;

    Ok(IndexHandle {
        writer,
        pool,
        vault_root: canonical,
        db_path: db_path.to_path_buf(),
    })
}

/// Collect read-only aggregate [`IndexStats`] for `vault_root`'s derived index
/// (`orgsidian index stats`). Resolves the DB path, refuses if the index is
/// absent, and reads through a fresh [`IndexPool`] — it does **not** call
/// [`open_index`], which drop-rebuilds on a version mismatch (wrong for
/// inspection), and it never spawns the writer.
///
/// # Errors
///
/// [`OrgError::Vault`] if the root cannot be resolved; [`OrgError::Index`] if
/// no index exists for the vault (run `index init` first) or a read fails.
pub async fn index_stats(vault_root: &Path) -> Result<IndexStats, OrgError> {
    let db_path = resolve_index_db_path(vault_root)?;
    if !db_path.exists() {
        return Err(index_absent_err(&db_path));
    }
    let pool = IndexPool::new(&db_path).map_err(index_err)?;
    pool.interact(collect_stats).await.map_err(index_err)
}

/// Run the read-only [`IntegrityReport`] checks for `vault_root`'s derived index
/// (`orgsidian index integrity`). Same read-only posture as [`index_stats`]:
/// resolve, refuse if absent, open just an [`IndexPool`]. A non-`ok` report is
/// a successful `Ok(report)` (the caller maps it to a non-zero exit); only a
/// resolve/absent/read failure is an `Err`.
///
/// # Errors
///
/// [`OrgError::Vault`] if the root cannot be resolved; [`OrgError::Index`] if
/// no index exists for the vault or a check cannot be run.
pub async fn index_integrity(vault_root: &Path) -> Result<IntegrityReport, OrgError> {
    let db_path = resolve_index_db_path(vault_root)?;
    if !db_path.exists() {
        return Err(index_absent_err(&db_path));
    }
    let pool = IndexPool::new(&db_path).map_err(index_err)?;
    pool.interact(check_integrity).await.map_err(index_err)
}

/// Explicitly rebuild `vault_root`'s derived index from scratch (LD-13 /
/// LD-49 / `orgsidian index rebuild`): delete the existing DB (plus its
/// `-wal`/`-shm` sidecars), then re-run the fresh create + full [`scan_vault`]
/// — reusing Story 3.6 wholesale. `cancel` and `progress` are forwarded to the
/// scan; the handle is shut down cleanly before returning.
///
/// # Errors
///
/// [`OrgError::Vault`] if the root cannot be resolved; [`OrgError::Io`] if a DB
/// file cannot be removed; [`OrgError::Index`] if the fresh create or scan
/// fails.
pub async fn rebuild_index(
    vault_root: &Path,
    cancel: &AtomicBool,
    progress: impl FnMut(ScanProgress),
) -> Result<ScanOutcome, OrgError> {
    let canonical = orgsidian_vault::canonicalize_vault_root(vault_root).map_err(vault_err)?;
    let db_path = default_index_db_path(&canonical)?;
    remove_index_files(&db_path)?;

    let handle = designate_vault(vault_root).await?;
    let outcome = scan_vault(&handle, cancel, progress).await;
    handle.shutdown().await;
    outcome
}

/// Implements FR-7 (Story 6.3 v0.1 Today Agenda subset): Scheduled-today +
/// Deadline-overdue-or-today for `vault_root`'s derived index. Same read-only
/// shape as [`index_stats`]/[`index_integrity`]: resolve the DB path, refuse
/// if the index is absent, read through a FRESH [`IndexPool`] rather than the
/// live [`IndexHandle`] — the Tauri command boundary hands this function a
/// `vault_root`, not the handle managed state holds.
///
/// `today` is the frontend's local calendar day (`YYYY-MM-DD`) — see
/// [`orgsidian_index::query::agenda::today`]'s docs for why the backend never
/// assumes a timezone.
///
/// # Errors
///
/// [`OrgError::Vault`] if the root cannot be resolved; [`OrgError::Index`] if
/// no index exists for the vault (run `index init` first) or the read fails.
pub async fn agenda_today(vault_root: &Path, today: &str) -> Result<Vec<AgendaItem>, OrgError> {
    let db_path = resolve_index_db_path(vault_root)?;
    if !db_path.exists() {
        return Err(index_absent_err(&db_path));
    }
    let pool = IndexPool::new(&db_path).map_err(index_err)?;
    let today = today.to_string();
    pool.interact(move |conn| orgsidian_index::query::agenda::today(conn, &today))
        .await
        .map_err(index_err)
}

/// The [`OrgError::Index`] returned when `stats`/`integrity` find no index for
/// the target vault — it names the missing path and points at `index init`,
/// and (per the read-only contract) is raised *without* creating a DB.
fn index_absent_err(db_path: &Path) -> OrgError {
    OrgError::Index {
        reason: format!(
            "no index found at {} — run `orgsidian index init <vault>` first",
            db_path.display()
        ),
    }
}

/// The environment override for the index base directory (Story 3.7). When set,
/// its value is used verbatim as the directory the derived index databases live
/// in, in place of `<data-dir>/orgsidian/index`. It makes the CLI integration
/// tests hermetic (`dirs::data_dir()` is not XDG-overridable on macOS) and
/// doubles as a CI / power-user knob for relocating the index store.
const DATA_DIR_ENV: &str = "ORGSIDIAN_DATA_DIR";

/// Resolve the derived index DB path for a canonical vault root:
/// `<index-base-dir>/index-<hash>.sqlite3`. The filename hashes the canonical
/// root (stable FNV-1a) so re-designating the same folder finds the same DB
/// (LD-40; the index lives outside the vault). The base directory is
/// [`index_base_dir`] (the `ORGSIDIAN_DATA_DIR` override or
/// `<data-dir>/orgsidian/index`).
fn default_index_db_path(canonical_root: &Path) -> Result<PathBuf, OrgError> {
    let dir = index_base_dir()?;
    std::fs::create_dir_all(&dir).map_err(|err| OrgError::Io {
        reason: format!("failed to create index directory {}: {err}", dir.display()),
    })?;
    Ok(dir.join(vault_db_filename(canonical_root)))
}

/// The directory the derived index databases live in: the [`DATA_DIR_ENV`]
/// override used verbatim when set, else `<os-data-dir>/orgsidian/index`
/// (LD-40 — the index lives OUTSIDE the vault).
fn index_base_dir() -> Result<PathBuf, OrgError> {
    if let Some(override_dir) = std::env::var_os(DATA_DIR_ENV) {
        return Ok(PathBuf::from(override_dir));
    }
    let mut dir = dirs::data_dir().ok_or_else(|| OrgError::Io {
        reason: "OS data directory is unavailable (no usable HOME/APPDATA)".to_string(),
    })?;
    for part in INDEX_SUBDIR {
        dir.push(part);
    }
    Ok(dir)
}

/// Resolve the derived index DB path for `vault_root` **without opening,
/// creating, or provisioning any directory** for it. Canonicalizes the vault
/// root (so it must exist) and returns where its index would live. Used by the
/// read-only `stats`/`integrity` commands to locate — and then existence-check
/// — the index; it deliberately does NOT go through [`default_index_db_path`]
/// (which `create_dir_all`s the base dir for the write paths), so a `stats`/
/// `integrity` run that then refuses via [`index_absent_err`] leaves the
/// filesystem untouched.
///
/// # Errors
///
/// [`OrgError::Vault`] if the vault root cannot be resolved; [`OrgError::Io`]
/// only if the OS data directory itself is unavailable (never from directory
/// creation — this function creates nothing).
pub fn resolve_index_db_path(vault_root: &Path) -> Result<PathBuf, OrgError> {
    let canonical = orgsidian_vault::canonicalize_vault_root(vault_root).map_err(vault_err)?;
    Ok(index_base_dir()?.join(vault_db_filename(&canonical)))
}

/// Stable per-vault DB filename: FNV-1a 64-bit over the canonical root's bytes.
/// Hand-rolled (not `DefaultHasher`, whose algorithm may change across Rust
/// releases) so the same folder maps to the same file across builds.
fn vault_db_filename(canonical_root: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in canonical_root.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("index-{hash:016x}.sqlite3")
}

/// Remove the index DB file and its WAL/SHM sidecars (drop-and-rebuild path).
/// A missing file is not an error.
fn remove_index_files(db_path: &Path) -> Result<(), OrgError> {
    for suffix in ["", "-wal", "-shm"] {
        let target = sidecar_path(db_path, suffix);
        match std::fs::remove_file(&target) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(OrgError::Io {
                    reason: format!("failed to remove {}: {err}", target.display()),
                });
            }
        }
    }
    Ok(())
}

/// `db_path` with `suffix` appended to its filename (`""`/`"-wal"`/`"-shm"`).
fn sidecar_path(db_path: &Path, suffix: &str) -> PathBuf {
    if suffix.is_empty() {
        return db_path.to_path_buf();
    }
    let mut name = db_path.as_os_str().to_os_string();
    name.push(OsString::from(suffix));
    PathBuf::from(name)
}

/// Prepend `vault_root` to `GlobalSettings.recent_vaults` (most-recent-first,
/// deduped, cap 10 — LD-40 / schema.rs:147-149). Delegated to the settings
/// store; the shell never writes settings directly.
fn push_recent_vault(vault_root: &Path) -> Result<(), OrgError> {
    let mut global = settings::read_global_settings().map_err(settings_err)?;
    global.recent_vaults.retain(|path| path != vault_root);
    global.recent_vaults.insert(0, vault_root.to_path_buf());
    global.recent_vaults.truncate(10);
    settings::write_global_settings(&global).map_err(settings_err)
}

/// Map a vault-layer failure into the IPC error type.
pub(crate) fn vault_err(err: VaultError) -> OrgError {
    OrgError::Vault {
        reason: err.to_string(),
    }
}

/// Map an index-layer failure into the IPC error type.
pub(crate) fn index_err(err: IndexError) -> OrgError {
    OrgError::Index {
        reason: err.to_string(),
    }
}

/// Map a settings-layer failure into the IPC error type.
fn settings_err(err: settings::SettingsError) -> OrgError {
    OrgError::Io {
        reason: err.to_string(),
    }
}
