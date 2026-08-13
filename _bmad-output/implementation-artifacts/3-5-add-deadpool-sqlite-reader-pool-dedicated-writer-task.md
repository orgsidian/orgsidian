# Story 3.5: Add `deadpool` reader pool + dedicated writer task

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 29

## Story

As the **user querying the Agenda or Search**,
I want a 4-reader connection pool plus a single dedicated writer task, so that concurrent reads never block each other and all writes serialize through one connection,
So that the NFR-3 (agenda <100ms) and NFR-4 (search <200ms) latency budgets are achievable and the index never corrupts under concurrency (LD-14, LD-7 at the index layer).

**Traces:** LD-14 (connection management — single dedicated writer task + reader pool, default size 4 — architecture.md:404), LD-16 (Tokio runtime; `spawn_blocking` for blocking/CPU-bound work — architecture.md:408), LD-4 (locked PRAGMAs, applied by `open()` from Story 3.3 — architecture.md:66), LD-7 (Single Writer Rule — the index layer's write path is the SQLite embodiment — architecture.md:69), FR-17 (SQLite derived index — architecture.md:1060), NFR-3 (agenda recompute <100ms — epics.md:98), NFR-4 (search <200ms — epics.md:100). Resolves the two **`busy_timeout` unset** deferred-work rows that Stories 3.3 and 3.4 explicitly assigned to Story 3.5 (`deferred-work.md` — the 3.3 row "…Owner: Story 3.5, which must set it as part of the pool's connection customizer" and the 3.4-review duplicate). Consumes, without editing, the Story 3.3 `open()` (LD-4 PRAGMAs) and the Story 3.4 `migrate()` (LD-12) seams.

## Scope Fence (read first)

This story adds the **concurrency plumbing** on top of the existing `open()` (Story 3.3) and `migrate()` (Story 3.4) primitives, inside `crates/orgsidian-index/`. The deliverable is: add `src/pool.rs` (a `ConnectionManager` implementing `deadpool::managed::Manager` + an `IndexPool` wrapping a `deadpool` pool of 4 reader connections), add `src/writer.rs` (a single dedicated writer task that owns the one writable, migrated connection and receives `IndexUpdate` messages over a `tokio::sync::mpsc` channel), grow `IndexError` with pool/writer variants, apply a `busy_timeout` to every connection this story creates, re-export the new surface from `lib.rs`, and add `tests/concurrency.rs` driving a **real on-disk DB** through 16 concurrent reads + serialized writes. It is **NOT**:

- **NOT `deadpool-sqlite`.** The epic AC and LD-14 name `deadpool-sqlite`, but it is **currently unusable here** and is deliberately **not** taken — see Dev Note 1 (maintainer-confirmed at story creation). `deadpool-sqlite 0.13` (latest, 2026-02-17) pins `rusqlite ^0.38`; the workspace is locked to `rusqlite 0.40` (Story 3.4 pinned it there for `rusqlite_migration 2.6`). Adding `deadpool-sqlite` would pull a **second** `rusqlite` (0.38) into the graph → `cargo deny check bans` (`multiple-versions = "deny"`) fails, and **LD-37 forbids ever putting `rusqlite` in `skip`/`skip-tree`** (`deny.toml:127-131`). Its pooled `Connection` would also be rusqlite-0.38's type, incompatible with `migrate(&mut rusqlite::Connection)` (0.40). The story uses the **generic `deadpool`** crate (no `rusqlite` edge) with a **local `Manager`** over our own rusqlite-0.40 `Connection` — same pool, same "deadpool" per LD-14's intent, single canonical `rusqlite`.
- **NOT the index-sync engine** (Story 3.6). Nothing here walks a Vault, calls the parser, or INSERTs real headline/tag/link/property/clock/link rows. `orgsidian-index` gains **no** dependency on `orgsidian-parser`. `IndexUpdate` is the *transport* for a unit of write-work (Dev Note 4), **not** a catalogue of concrete index mutations — 3.6 defines those and sends them through this channel unchanged.
- **NOT the query API** (Epics 7/8). No `src/query/`, no `IndexQuery` trait, no agenda/search/backlink `SELECT`s, no FTS5 querying. The pool hands out connections and offers a blocking-safe execution helper; it ships **no** domain queries. `tests/concurrency.rs`'s reads are trivial (`SELECT 1`, `SELECT count(*)`), asserting *concurrency behaviour*, not query results.
- **NOT editing `open()`, `apply_schema`, or `migrate`.** `crates/orgsidian-index/src/connection.rs` and `src/migrations.rs` are **consumed, not modified**. `busy_timeout` is applied by *this story's* code (the pool's `create` and the writer's init) via `rusqlite::Connection::busy_timeout`, **not** folded into `open()` — the deferred-work owner note assigns it to "the pool's connection customizer", and the writer connection (not pooled) needs it too, so a shared `BUSY_TIMEOUT` const applied at both new sites is the fix. `open()` stays byte-unchanged, so `tests/schema.rs` and its PRAGMA assertions stay green untouched (Dev Note 3).
- **NOT folding `migrate` into `open()` or the reader path.** Per Story 3.4 Dev Note 5, the reader pool must **never** run migrations — readers open a schema that already exists. Exactly one site migrates: the **writer**, once, at construction, before it serves any write and before reads are valid (Dev Note 5). `ConnectionManager::create` calls `open()` **only** (+ `busy_timeout`).
- **NOT the `application_id` / foreign-file guard, drift/rebuild reaction, or path-identity policy.** Those remain Story 3.6 (per `deferred-work.md` — the `[PARTIALLY ADDRESSED — Story 3.4]` row and the path-identity row). This story adds no schema-identity check to `open()`; it assumes the path handed to it is the vault's index.
- **NOT read-only reader connections.** Readers open read-**write**-capable connections (via `open()`, which must set `journal_mode=WAL` and therefore needs write access). The Single Writer Rule at this layer is a *discipline* — all mutations route through the writer task — not enforced by `SQLITE_OPEN_READ_ONLY` reader handles. Read-only reader hardening is recorded as deferred (Dev Note 7), not built.
- **NOT rebuild / integrity / CLI** (LD-13 → Stories 3.6/3.7). `tests/failure_modes.rs`, `tests/failure_modes_coverage.rs`, `docs/failure-modes/coverage-matrix.md` stay **byte-untouched** (`EXPECTED_REMAINING_PLACEHOLDERS` unchanged).
- **NOT a cross-crate edge.** `orgsidian-index` is a LEAF (`deny.toml:193-195` — only `orgsidian-core` may wrap it). Nothing consumes it yet, so **do not** add an `orgsidian-index` entry to `[workspace.dependencies]` and **do not** touch `orgsidian-core`, the shell, or the CLI.
- **NOT sentinel turf.** Byte-untouched: every `crates/*/tests/anchor.rs`, all of `crates/orgsidian-vault/`, `crates/orgsidian-parser/`, `crates/orgsidian-watcher/`, `.github/workflows/*`, and `_bmad-output/planning-artifacts/architecture.md` (process archive — LD-14's `deadpool-sqlite` wording goes stale by design; record it, do not edit the archive — Dev Note 1). `deny.toml` is touchable **only** if `cargo deny check bans` actually fires on a duplicate version from the `deadpool`/`tokio` trees — and then only with a matching ledger row (AC1).

## Acceptance Criteria

### AC1 — Dependency delta is generic `deadpool` + `tokio`; **no** `deadpool-sqlite`; the single-`rusqlite`/single-`tokio` invariants (LD-37) hold.

- **The pool crate is generic `deadpool`, not `deadpool-sqlite`** (Dev Note 1, maintainer-confirmed). `[workspace.dependencies]` gains `deadpool = { version = "0.13", default-features = false, features = ["managed", "rt_tokio_1"] }` and `tokio = { version = "1", default-features = false }`, each with the established story-attributed comment block (match the `rusqlite`/`rusqlite_migration` house style in the root `Cargo.toml`: story number, LD trace, why this version, license, and — for `tokio` — the LD-37 canonical-version coupling).
- **`tokio` is version-locked to the one already resolved.** `tokio 1.52.3` is already in `Cargo.lock` (Tauri's). Pin `"1"` so Cargo unifies on the existing minor — **no** second `tokio` major/duplicate (LD-37 names `tokio` an explicit canonical-version invariant — architecture.md:1168, `deny.toml:127-131`). Confirm `cargo tree -p orgsidian-index -i tokio` shows exactly one `tokio` and `deny check bans` stays green.
- **`deadpool` has no `rusqlite` edge.** Verified on crates.io (2026-08-13): generic `deadpool 0.13.0` depends on `deadpool-runtime ^0.3`, `num_cpus`, and `tokio` (feature `sync`) — **no** `rusqlite`, so the rusqlite-0.40 pin is untouched. License MIT OR Apache-2.0 (allowlist-compatible; verify in `cargo deny check licenses`). Enable **only** `managed` + `rt_tokio_1`; do **not** pull `deadpool-sqlite`, `serde`, `r2d2`, `mobc`, or `async-trait` (deadpool 0.13 uses native async-fn-in-traits — no `async-trait` crate).
- `crates/orgsidian-index/Cargo.toml` consumes `deadpool = { workspace = true }` and `tokio = { workspace = true, features = ["sync", "rt"] }` (normal deps); `[dev-dependencies]` adds `tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }` for `#[tokio::test]`. Features are additive, so the test binary gets `sync + rt + macros + rt-multi-thread`.
- Transitive new-crate delta is expected to be `deadpool`, `deadpool-runtime`, and possibly `num_cpus` (likely already in the tree via another dep — confirm). Record the **actual** delta from `cargo tree -p orgsidian-index` + the `Cargo.lock` diff in Completion Notes; confirm none of them introduce a **duplicate major** of an existing crate.
- `Cargo.lock` is committed (CI runs `--locked`; a stale lock fails the build).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked`, `cargo deny --locked check all`, `cargo audit` all green.
- **If `cargo deny check bans` reports a duplicate major** from the `deadpool`/`tokio` trees: add a `[[bans.skip]]` (or `skip-tree`) entry with `reason = "Story 3.5: …"` **and** the matching row in `docs/security/advisory-exceptions.md` (CI's `scripts/check-allowlist-sync.mjs` fails on ledger drift). `rusqlite`/`tokio`/`serde`/`chrono` must **never** be added to `skip`/`skip-tree` (`deny.toml:127-131` — LD-37 invariant names them explicitly). If nothing fires (the likely case), `deny.toml` stays untouched.
- Commitlint-conformant conventional commit (`feat(index): …`); plain message, no AI-credit trailers.

### AC2 — `src/pool.rs`: `ConnectionManager` + `IndexPool` over a `deadpool` pool of 4 reader connections, each carrying the LD-4 PRAGMAs **and** a `busy_timeout`.

- `struct ConnectionManager { db_path: PathBuf }` implements `deadpool::managed::Manager` with `type Type = rusqlite::Connection;` and `type Error = IndexError;`.
  - `async fn create(&self) -> Result<Connection, IndexError>` calls `connection::open(&self.db_path)?` (the LD-4 locked PRAGMA set — **reused verbatim**, not reimplemented) and then applies `conn.busy_timeout(BUSY_TIMEOUT)?`. It does **not** call `migrate` (Scope Fence; readers never migrate). Because `open()` is blocking rusqlite, wrap it in `tokio::task::spawn_blocking` inside `create` so pool warm-up never blocks an async worker thread (LD-16).
  - `async fn recycle(&self, conn, _: &Metrics) -> RecycleResult<IndexError>` performs a cheap liveness probe (e.g. `conn.execute_batch("SELECT 1")` via `spawn_blocking`, or `pragma_query`) and returns `Err` to have `deadpool` discard and re-`create` a poisoned connection. Keep it minimal — no re-running of PRAGMAs.
- `struct IndexPool { pool: deadpool::managed::Pool<ConnectionManager> }` with `pub fn new(db_path: &Path) -> Result<IndexPool, IndexError>` building `Pool::builder(ConnectionManager { … }).max_size(DEFAULT_READERS).runtime(Runtime::Tokio1).build()` and mapping `BuildError` into `IndexError` (AC-specified variant, AC4-tested-adjacent). `const DEFAULT_READERS: usize = 4;` — a named const, LD-14's "default size 4", with a doc comment tying it to LD-14 + NFR-3/NFR-4.
- `busy_timeout` value: pick a concrete `const BUSY_TIMEOUT: Duration` with a documented rationale now that there is contention to justify it (a pool + a writer whose WAL checkpoint or write transaction routinely overlaps a read — the exact situation the deferred-work rows anticipated). A comment states the value and why (e.g. long enough to ride out a WAL checkpoint, short enough to surface a genuine deadlock rather than hang). Do **not** leave it at SQLite's default of `0` (fail-immediately), which is the defect being resolved.
- **Blocking-safe read execution helper (mirrors `deadpool-sqlite::interact`, no SQL baked in):** `IndexPool::interact<F, R>(&self, f: F) -> Result<R, IndexError>` where `F: FnOnce(&Connection) -> Result<R, IndexError> + Send + 'static, R: Send + 'static`, which `get()`s a pooled connection and runs `f` inside `tokio::task::spawn_blocking` (so blocking rusqlite never runs on an async worker thread — LD-16), returning the connection to the pool on completion. Pool-acquire failures map into `IndexError` (AC-specified variant). This is the *mechanism* NFR-3/NFR-4 reads will use; it ships **no** domain queries.
- No `unwrap`/`expect`/`panic!` in committed non-test code. `println!` forbidden.

### AC3 — `src/writer.rs`: one dedicated writer task owns the single writable, migrated connection and serializes all writes received over an `mpsc` channel.

- `struct IndexWriter { tx: tokio::sync::mpsc::Sender<IndexUpdate>, handle: JoinHandle<()> }` (or equivalent), created by `pub fn spawn(db_path: &Path) -> Result<IndexWriter, IndexError>` which:
  1. opens the **one** writable connection via `connection::open(db_path)?`, applies `busy_timeout(BUSY_TIMEOUT)?`, then runs `migrate(&mut conn)?` **once** — this is the sole migration site in the running system (LD-12; Story 3.4 Dev Note 5). Migrating here, before the task loop starts, is what makes subsequent reader queries valid (Dev Note 5).
  2. spawns the writer loop on a **dedicated `tokio::task::spawn_blocking` task** that owns `conn` and drains the channel with `rx.blocking_recv()`. Rationale (doc it): `rusqlite::Connection` is `Send + !Sync` and every call blocks; running the loop on `spawn_blocking` keeps it on Tokio's blocking-thread pool and off the async worker threads (LD-16). A plain `async` task calling blocking rusqlite would stall a worker thread — wrong.
- `IndexUpdate` is the **minimal write-work transport** (Dev Note 4), **not** a catalogue of concrete index mutations: it carries a boxed write-thunk `Box<dyn FnOnce(&mut Connection) -> Result<(), IndexError> + Send>` plus a `tokio::sync::oneshot::Sender<Result<(), IndexError>>` ack. Story 3.6 sends its real sync SQL through this channel unchanged; this story defines **no** headline/tag/link operations.
- `pub async fn execute<F>(&self, f: F) -> Result<(), IndexError>` where `F: FnOnce(&mut Connection) -> Result<(), IndexError> + Send + 'static` sends the thunk down the channel and awaits the `oneshot` result. This is the **Single Writer Rule at the index layer** (LD-7/LD-14): every mutation is serialized through the one connection, so two concurrent `execute` calls can never race a write. A send failure (the writer task is gone) or a canceled `oneshot` maps to a distinct `IndexError` variant (AC-specified) so callers can tell "index writer unavailable" from a SQL error.
- **Graceful shutdown:** dropping `IndexWriter` (hence its `Sender`) closes the channel; `blocking_recv()` returns `None`; the loop drops `conn` and exits. Document this. Optionally expose the `JoinHandle` (or an `async fn shutdown(self)`) so a consumer can await drain; the drop path must not panic or leak the blocking thread.
- No `unwrap`/`expect`/`panic!` in committed non-test code.

### AC4 — `tests/concurrency.rs` drives a real on-disk DB and proves the concurrency contract.

- Tests live in `crates/orgsidian-index/tests/concurrency.rs` (new integration file). Use a **real on-disk temp DB** (`tempfile::TempDir` + a `.db` path — the 3.3/3.4 rule; **never `:memory:`** for WAL/pool/`busy_timeout` semantics, and an in-memory DB is per-connection so a pool of them wouldn't even share state). Tests are `#[tokio::test(flavor = "multi_thread", worker_threads = …)]` so `spawn_blocking` and true concurrency are available.
- Coverage required:
  - **16 concurrent reads on a 4-reader pool complete without deadlock or pool exhaustion** (the epic AC's headline): first bring the schema up (spawn the `IndexWriter`, which migrates), then spawn **16** async tasks each doing `pool.interact(|c| { c.query_row("SELECT 1", …) })`, `join_all`, and assert **all 16** return `Ok`. `deadpool` queues `get()` calls beyond `max_size`, so the property is *no deadlock, no `PoolError` exhaustion, all reads eventually served* by only 4 connections. After the join, assert the pool `status()` reports all 4 connections available again (they were returned, not leaked).
  - **The writer serializes concurrent writes:** submit N (e.g. 32) concurrent `writer.execute(|c| …)` calls that each mutate shared state (e.g. `INSERT`/`UPDATE` a scratch row in an existing table such as `vault_meta`, or bump a counter), `join_all`, assert **all** `Ok`, **no `SQLITE_BUSY`**, and the final aggregate is exactly consistent with N applications (proves serialization, not lost updates).
  - **Reads interleaved with writes:** run reads (`pool.interact`) concurrently with writer `execute`s and assert no deadlock and that a read after a write observes the committed state (WAL readers + single writer compose).
  - **`busy_timeout` is actually set on pooled connections:** `pool.interact(|c| c.query_row("PRAGMA busy_timeout", …))` returns the configured value (proves the customizer ran — the deferred-work resolution is behaviourally verified, not just asserted in a comment).
  - **Anti-placebo (Story 1.9 discipline):** at least one assertion must fail under a plausible mutation. Verify by temporarily (a) building the pool with `max_size(1)` **and** having `interact` hold the connection across a barrier so 16 reads would deadlock/time out → the concurrency test must fail; **or** (b) not spawning the writer / dropping the ack so an `execute` never completes → the write test must fail; **or** (c) setting `busy_timeout(Duration::ZERO)` → the busy_timeout assertion must fail. Restore and re-run green. Record the mutation check in the Debug Log.
- No `unwrap`/`expect`/`panic!` in committed non-test code; tests may `.unwrap()` freely.

### AC5 — Scope fence holds, deferred-work is updated, every gate stays green.

- `git diff main...HEAD --name-only` (branch-scoped — `git diff HEAD` is non-probative for committed work, per Story 3.2's review) shows only: root `Cargo.toml`, `Cargo.lock`, `crates/orgsidian-index/Cargo.toml`, `src/pool.rs` (new), `src/writer.rs` (new), `src/lib.rs` (re-exports + crate-doc sentence), `src/error.rs` (new pool/writer variants), `tests/concurrency.rs` (new), `_bmad-output/implementation-artifacts/deferred-work.md`, this story file, `sprint-status.yaml` — plus, **only if** a duplicate-version ban or an audit advisory actually fired, `deny.toml` + `docs/security/advisory-exceptions.md`.
- Sentinels verified byte-untouched (list in the Scope Fence). `connection.rs`, `migrations.rs`, `tests/schema.rs`, `tests/migrations.rs`, `architecture.md`, `migrations/0001_initial-schema.sql` unchanged. `EXPECTED_REMAINING_PLACEHOLDERS` unchanged.
- Deferred-work rows filed/updated per Dev Note 7. The two **`busy_timeout` unset** rows are marked **RESOLVED by Story 3.5**.
- All gates green (AC1 list).

## Tasks / Subtasks

- [x] Task 1 — Dependencies (AC1)
  - [x] Root `Cargo.toml`: add `deadpool = { version = "0.13", default-features = false, features = ["managed", "rt_tokio_1"] }` and `tokio = { version = "1", default-features = false }` to `[workspace.dependencies]`, each with a story-attributed comment block (LD trace, version rationale, license; tokio's notes the LD-37 canonical-version coupling)
  - [x] `crates/orgsidian-index/Cargo.toml`: `deadpool = { workspace = true }`, `tokio = { workspace = true, features = ["sync", "rt"] }`; `[dev-dependencies]` `tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }`
  - [x] `cargo build --workspace --locked`; commit `Cargo.lock`; `cargo tree -p orgsidian-index` + `-i tokio` + `-i rusqlite` — record the transitive delta and prove exactly one `tokio` and one `rusqlite`
- [x] Task 2 — Reader pool (AC2)
  - [x] `src/pool.rs`: `ConnectionManager` (`create` = `open()` + `busy_timeout`, via `spawn_blocking`; `recycle` = cheap liveness probe); `IndexPool::new` builds a `deadpool` pool with `max_size(DEFAULT_READERS=4)` + `Runtime::Tokio1`
  - [x] `const BUSY_TIMEOUT` with documented rationale; applied to every connection this story creates
  - [x] `IndexPool::interact<F,R>` blocking-safe read helper (`get()` + `spawn_blocking`), no domain SQL
- [x] Task 3 — Writer task (AC3)
  - [x] `src/writer.rs`: `IndexWriter::spawn` opens the one writable conn, `busy_timeout`, `migrate` **once**, then spawns the dedicated `spawn_blocking` loop draining `mpsc` via `blocking_recv`
  - [x] `IndexUpdate` = boxed write-thunk + `oneshot` ack; `IndexWriter::execute` sends + awaits; distinct writer-gone error path; graceful drop/shutdown
- [x] Task 4 — Errors + re-exports (AC2, AC3)
  - [x] `src/error.rs`: add non-panicking variants for pool build, pool acquire, and writer-unavailable (map `deadpool` `BuildError`/`PoolError` and channel/oneshot failures) on the existing `#[non_exhaustive] IndexError`
  - [x] `src/lib.rs`: `pub mod pool; pub mod writer;` + re-exports (`IndexPool`, `IndexWriter`, `IndexUpdate` as appropriate); present-tense crate-doc sentence describing the shipped pool + writer (no forward-looking prose)
- [x] Task 5 — Tests (AC4)
  - [x] `tests/concurrency.rs`: 16 concurrent reads (no deadlock/exhaustion, pool returns to full), writer serialization (32 writes, no `SQLITE_BUSY`, exact aggregate), reads-interleaved-with-writes, `busy_timeout` read-back
  - [x] Anti-placebo mutation check (one of: max_size(1)+barrier, writer-not-spawned, busy_timeout=0); restore green; note in Debug Log
- [x] Task 6 — Gates + hygiene (AC1, AC5)
  - [x] fmt / clippy / build / test / deny / audit
  - [x] `git diff main...HEAD --name-only` scope check; sentinels + `connection.rs`/`migrations.rs`/`tests/schema.rs`/`tests/migrations.rs`/`architecture.md` untouched; `EXPECTED_REMAINING_PLACEHOLDERS` unchanged
  - [x] Update `deferred-work.md`: mark both `busy_timeout` rows RESOLVED; file the LD-14 `deadpool-sqlite`→`deadpool` wording deviation, the read-only-reader hardening, the bounded-channel/backpressure follow-up, and the `deadpool`↔`rusqlite` re-check-on-bump coupling (Dev Note 7)

## Dev Notes

### 1. Why generic `deadpool` and not `deadpool-sqlite` (the central decision — maintainer-confirmed)

The epic AC (epics.md:962) and LD-14 (architecture.md:404) both name `deadpool-sqlite`. It is **currently unusable** in this workspace, and the maintainer confirmed the pivot to generic `deadpool` at story creation (2026-08-13):

| crate | latest | pins `rusqlite` | verdict |
|---|---|---|---|
| `deadpool-sqlite` | 0.13.0 (2026-02-17) | `^0.38` (→ `<0.39`) | ✗ duplicates rusqlite → LD-37 ban |
| `deadpool` (generic) | 0.13.0 (2026-02-17) | *(none)* | ✓ we supply the `Manager` |

The workspace is on `rusqlite 0.40` (0.40.0 shipped 2026-05-26), locked there by `rusqlite_migration 2.6` in Story 3.4. `deadpool-sqlite 0.13` predates rusqlite 0.40 and still requires `rusqlite ^0.38`. Adding it would put a **second** `rusqlite` in the graph — `cargo deny check bans` has `multiple-versions = "deny"`, and `deny.toml:127-131` (LD-37, architecture.md:1168) makes it a **binding rule** that `rusqlite` is never added to `skip`/`skip-tree`. The types wouldn't even unify: a `deadpool-sqlite` pooled `Connection` is rusqlite-0.38's, incompatible with `migrate(&mut rusqlite::Connection)` (0.40).

`deadpool-sqlite` is a thin convenience wrapper: a `Manager<rusqlite::Connection>` + an `interact()` helper (via `deadpool-sync`). We reproduce exactly that with the **generic `deadpool`** crate — which has **no `rusqlite` dependency** (its deps are `deadpool-runtime`, `num_cpus`, `tokio` feature `sync`; verified crates.io 2026-08-13) — plus a ~30-line local `Manager` over our own rusqlite-0.40 `Connection`. This keeps "pool via deadpool, default size 4" per LD-14's **intent**, preserves the single canonical `rusqlite`, and keeps Story 3.4 intact.

**Disclosed deviation:** LD-14's literal "`deadpool-sqlite`" is now stale. Handle it **exactly** as Story 3.4 handled the LD-11 `sql/schema.sql` wording: a Completion Note + a `deferred-work.md` row requesting an `architecture.md` addendum. **Do not edit `architecture.md`** (process archive). This mirrors the `rusqlite_migration` lock-step lesson from Story 3.4 — the SQLite ecosystem's pool/migration wrappers lag `rusqlite` minors, so the workspace either waits for them or (as here) depends one layer down and supplies the glue.

### 2. `deadpool` (generic, 0.13) API surface used

Verified against the deadpool docs (fetched 2026-08-13):

- `deadpool::managed::Manager` — native async-fn-in-traits (deadpool 0.12+; **no `async-trait` crate**). `type Type; type Error; async fn create(&self) -> Result<Type, Error>; async fn recycle(&self, obj: &mut Type, metrics: &Metrics) -> managed::RecycleResult<Error>;`.
- `deadpool::managed::Pool<M>` built via `Pool::builder(mgr).max_size(n).runtime(Runtime::Tokio1).build()` → `Result<Pool, BuildError>`. `Runtime::Tokio1` requires the `rt_tokio_1` feature and enables `get()` timeouts + runtime-aware behaviour.
- `pool.get().await -> Result<Object<M>, PoolError<M::Error>>`. `Object<M>` is an RAII guard that `Deref`/`DerefMut`s to `M::Type` and returns the connection to the pool on drop. It is `Send` when `M::Type` is `Send` (`rusqlite::Connection` is `Send`), so an `Object` can be moved into `spawn_blocking`.
- `pool.status() -> Status { max_size, size, available, … }` — used by the test to assert connections were returned.
- `PoolError` / `BuildError` are the two failure types to map into `IndexError`. `PoolError<E>` nests `M::Error` (= `IndexError`); avoid a recursive `#[from]` — map it into a message-bearing variant instead.

`rusqlite::Connection` is `Send + !Sync`. The pool moving whole connections between tasks is fine (Send); what is **not** fine is running a blocking query on an async worker thread — hence `spawn_blocking` for every actual rusqlite call (LD-16).

### 3. `busy_timeout` — resolving the deferred rows without touching `open()`

Two deferred-work rows assign `busy_timeout` to this story: the Story 3.3 row ("…there is no `busy_timeout`, so a connection that finds the database locked fails immediately with `SQLITE_BUSY`… Owner: Story 3.5, which must set it as part of the pool's connection customizer so every pooled connection carries it") and the Story 3.4-review duplicate ("…cross-connection serialization is Story 3.5's single-writer model"). Story 3.5 is where the contention that justifies a timeout first exists: a 4-reader pool plus a writer whose WAL checkpoint or write transaction routinely overlaps a read.

Apply it via `rusqlite::Connection::busy_timeout(Duration)` at the **two sites this story creates connections** — `ConnectionManager::create` (every pooled reader) and `IndexWriter::spawn` (the one writer) — using a shared `const BUSY_TIMEOUT`. Do **not** fold it into `connection.rs::open()`: the deferred-work owner note scopes it to "the pool's connection customizer", `open()` is Story 3.3 turf that Story 3.4 was careful to leave byte-unchanged, and editing it would force new assertions into `tests/schema.rs` (which must stay green untouched — the same discipline Story 3.4 kept). If a future story wants `busy_timeout` on *every* connection unconditionally, that is a deliberate 3.3 PRAGMA follow-up, recorded but not taken here.

Pick the value with a stated rationale (a WAL checkpoint on a small index is milliseconds; a few seconds rides out realistic contention while still surfacing a genuine deadlock rather than hanging forever). Assert it back in the test so the resolution is behavioural, not a comment.

### 4. `IndexUpdate` is transport, not a mutation catalogue

The AC says the writer "receiv[es] `IndexUpdate` messages via an `mpsc` channel." This story ships the **channel and the loop**, not the domain operations (that is Story 3.6, the sync engine — explicitly out of scope). So `IndexUpdate` carries a **unit of write-work** rather than enumerating concrete mutations:

```
struct IndexUpdate {
    thunk: Box<dyn FnOnce(&mut rusqlite::Connection) -> Result<(), IndexError> + Send>,
    ack:   tokio::sync::oneshot::Sender<Result<(), IndexError>>,
}
```

`IndexWriter::execute(f)` boxes `f`, sends it, and awaits the `oneshot`. This is the command pattern `deadpool-sqlite::interact` uses for reads, applied to the single-writer path. Story 3.6 calls `execute(|conn| { /* real INSERT/UPDATE/DELETE + FTS sync */ })` and never touches the transport. If 3.6 later prefers a concrete `enum IndexUpdate { UpsertFile(…), DeleteFile(…), … }`, the swap is internal to the writer — the point of shipping the plumbing now is that the concurrency contract (serialization, no `SQLITE_BUSY`, backpressure) is proven before any real sync SQL exists. Keep the boxed-thunk shape unless the dev finds a cleaner minimal transport; either way, **no** headline/tag/link operations here.

### 5. Init order: the writer migrates once, before reads are valid; readers never migrate

Per Story 3.4 Dev Note 5, `migrate` must **not** run on the read path — readers open a schema that already exists, and coupling migration into `open()` (or into `ConnectionManager::create`) would push it onto every reader. Exactly one site migrates: `IndexWriter::spawn`, once, before the loop starts. That ordering is also a **correctness** requirement: on a fresh path, a reader that opens before the writer migrates sees a schema-less DB and its `SELECT` fails with "no such table". So a consumer must construct the writer (migrate) **before** issuing reads.

Recommended (not strictly required by the AC, but it removes the footgun and is how Story 3.6 will consume this): a thin facade — e.g. `struct Index { pool: IndexPool, writer: IndexWriter }` with `Index::open(db_path)` that (1) spawns the writer (opens + `busy_timeout` + `migrate`), then (2) builds the reader pool, and holds both — establishing the migrate-then-serve order in one place. If you add it, keep it minimal (construction + accessors); it is **not** a query API. If you don't, document the ordering requirement loudly on `IndexPool`/`IndexWriter` so 3.6 can't reads-before-migrate by accident.

### 6. Concurrency test: what "no deadlock or pool exhaustion" actually asserts

With `max_size = 4` and 16 concurrent `get()`s, `deadpool` **queues** the excess — it does not error. The bug this guards against is a connection that is acquired and never returned (a leaked `Object`, or an `interact` that holds it across an await that itself needs a connection → self-deadlock). So the test's real assertions are: (a) all 16 reads return `Ok` (none starved), (b) after `join_all`, `pool.status().available == DEFAULT_READERS` (every connection came back). Use `flavor = "multi_thread"` so the 16 tasks and the `spawn_blocking` bodies genuinely overlap — a current-thread runtime would serialize them and hide a real deadlock. For the writer, 32 concurrent `execute`s that each bump a counter must sum to exactly 32 (serialization → no lost updates) with **zero** `SQLITE_BUSY` (the writer is the only writer, so it never contends with itself; `busy_timeout` covers the reader-vs-checkpoint overlap).

### 7. Deferred-work entries to file/update (`_bmad-output/implementation-artifacts/deferred-work.md`)

Follow the file's row format (bold summary, `[path]`, severity, rationale, owner):

- **Mark RESOLVED** both **`busy_timeout` unset** rows (the Story 3.3 row and the Story 3.4-review duplicate) — Story 3.5 sets `busy_timeout` on every reader (`ConnectionManager::create`) and on the writer connection (`IndexWriter::spawn`) via a shared `BUSY_TIMEOUT` const, verified by a `PRAGMA busy_timeout` read-back test.
- **LD-14 wording: pool is generic `deadpool` + a local `Manager`, not `deadpool-sqlite`** [`architecture.md:404`] [LOW] — `deadpool-sqlite 0.13` pins `rusqlite ^0.38`, incompatible with the workspace's `rusqlite 0.40`; generic `deadpool` (no rusqlite edge) is used instead. Architecture addendum requested; archive not edited (per the 3.4 LD-11 precedent).
- **`deadpool`/`deadpool-sqlite` ↔ `rusqlite` version re-check on the next `rusqlite` bump** [`Cargo.toml`] [LOW] — when `rusqlite` is next bumped, re-check whether a `deadpool-sqlite` release has caught up to that minor; if so, a future story may collapse the local `Manager` back onto `deadpool-sqlite`. Couples loosely like the `rusqlite_migration` lock-step (`[[feedback_version_policy]]`). Owner: whoever next bumps `rusqlite`.
- **Read-only reader connections not enforced** [`crates/orgsidian-index/src/pool.rs`] [LOW] — pooled readers open read-write-capable connections (via `open()`, which needs write access to set `journal_mode=WAL`); the Single Writer Rule at this layer is a discipline (all writes route through `IndexWriter`), not enforced by `SQLITE_OPEN_READ_ONLY` handles. Hardening to genuinely read-only reader connections (requires an `open()` variant that assumes WAL is already set) is deferred. Owner: a later index-hardening pass.
- **Writer channel is unbounded/backpressure policy unset** [`crates/orgsidian-index/src/writer.rs`] [LOW] — if the `mpsc` is created unbounded (or its bound and drop-vs-block policy is not yet calibrated), a flood of `execute` calls could grow memory unboundedly before the single writer drains them. Fine at notes scale and with `execute` awaiting each ack (natural backpressure per caller), but the bound/backpressure policy for a bulk re-index (Story 3.6's initial scan) wants calibration against real load. Owner: Story 3.6 (initial-scan indexer), which is the first bulk writer.
- **Writer has no `application_id`/foreign-file guard** — unchanged; still Story 3.6 (already captured by the `[PARTIALLY ADDRESSED — Story 3.4]` row; the writer opening the one connection does not add the guard).

### 8. Code conventions (established — follow exactly)

- Module doc header naming LD/FR traces (Stories 1.17/1.18/3.1/3.2/3.3/3.4 precedent): LD-14, LD-16, LD-7, FR-17, NFR-3/NFR-4 for `pool.rs` and `writer.rs`.
- `lib.rs` = declarations + re-exports only, no logic (architecture.md:738).
- `error.rs` owns `IndexError`, `#[non_exhaustive]`, `thiserror::Error` derive — new variants mirror the existing `Sqlite`/`Migration`/`Pragma` shapes; keep them non-panicking.
- No `unwrap`/`expect`/`panic!` in committed non-test code; `println!` forbidden (use `tracing` if ever needed — not needed here).
- **No forward-looking prose in rustdoc** (standing rule since Story 3.2): describe what shipped; phrase intent as intent. The crate-doc mention of the query API stays intent-shaped.
- Assert *behavior*, not text: open a real DB, run genuinely concurrent reads/writes, read `busy_timeout` and `pool.status()` back (Story 1.9 anti-placebo).

### Project Structure Notes

- New: `crates/orgsidian-index/src/pool.rs`, `crates/orgsidian-index/src/writer.rs`, `crates/orgsidian-index/tests/concurrency.rs`. Modified: `crates/orgsidian-index/src/lib.rs` (module decls + re-exports + one crate-doc sentence), `src/error.rs` (pool/writer variants), `crates/orgsidian-index/Cargo.toml`, root `Cargo.toml`, `Cargo.lock`.
- One concern per module (architecture.md:738-742): `pool.rs` = reader pool + manager; `writer.rs` = writer task + `IndexUpdate` + `execute`. If a facade is added (Dev Note 5), it is a small `index.rs`/`lib.rs`-level struct, not a query surface.
- Fixtures: none new. `tempfile` dev-dep already present (Story 3.3).
- Branch per repo convention: `story/3.5-reader-pool-writer-task` off `main`; conventional commits (`feat(index): …`; commitlint gate); plain messages, no AI-credit trailers. GitHub issue **#29**; label `status:in-review` during code review. Merge via `gh pr merge --admin` (branch protection requires 1 review, unsatisfiable solo — `[[project_orgsidian_github_plan]]`).

### Testing Standards Summary

- Integration tests at `crates/orgsidian-index/tests/concurrency.rs` (architecture.md:723-729); the pool/writer paths need a **real on-disk temp DB** (`open()` + `.db`) and a **multi-thread** Tokio runtime (`#[tokio::test(flavor = "multi_thread")]`) so concurrency is real.
- Deterministic: no timing assertions on wall-clock durations; assert *outcomes* (all reads `Ok`, exact write aggregate, pool returned to full, `busy_timeout` value). A `TempDir` per test; no shared DB file across tests.
- Anti-placebo mutation check mandatory (AC4) — at least one assertion must fail under a plausible mutation (max_size(1)+barrier, writer-not-spawned, or busy_timeout=0).
- The perf gate (`assert_no_perf_regression!`) does **not** apply here — NFR-3/NFR-4 budgets are assigned to Stories 7.1/8.4 (`docs/perf/targets.md`), which measure the actual agenda/search queries. This story ships the *connection model* those budgets depend on, not the queries; do **not** add a perf baseline for this crate.

### Latest Technical Information

- **Generic `deadpool` 0.13.0** (crates.io, verified 2026-08-13). MIT OR Apache-2.0. Deps: `deadpool-runtime ^0.3`, `num_cpus`, `tokio` (feature `sync`) — **no `rusqlite`**. Features to enable: `managed` (the pool) + `rt_tokio_1` (`Runtime::Tokio1`). Uses native async-fn-in-traits (no `async-trait`).
- **`deadpool-sqlite 0.13.0`** (verified 2026-08-13) requires `rusqlite ^0.38` — the reason it is **not** used (Dev Note 1). `rusqlite 0.40.0` shipped 2026-05-26; the wrapper hasn't caught up.
- **`tokio 1.52.3`** already in `Cargo.lock` (Tauri). Pin `"1"` to unify on it — LD-37 canonical-version invariant (no duplicate `tokio` major — `deny.toml:127-131`, architecture.md:1168). Features: `sync` (`mpsc`, `oneshot`), `rt` (`spawn_blocking`), plus dev `macros` + `rt-multi-thread`.
- **`rusqlite::Connection`** is `Send + !Sync`; `busy_timeout(Duration)` is a built-in method (no PRAGMA-string needed to *set* it, though the test reads it back via `PRAGMA busy_timeout`). Blocking — every rusqlite call must run under `spawn_blocking` off the async worker threads (LD-16).
- **`tokio::sync::mpsc::Receiver::blocking_recv`** lets the `spawn_blocking` writer loop drain the async channel from a blocking context; senders use async `send().await`. `tokio::sync::oneshot` carries the per-write ack.
- Rust: workspace `edition = "2021"`, `rust-toolchain.toml` channel `stable` (unpinned). Native async-fn-in-traits (deadpool 0.13, AFIT) needs Rust ≥ 1.75 — comfortably below current stable; no MSRV floor introduced beyond what's already in use.

### References

- Epic AC source: `_bmad-output/planning-artifacts/epics.md` §Epic 3 → Story 3.5 (lines 952-964); Epic 3 summary (line 364); neighbouring Stories 3.4 (937), 3.6 (966), 3.7 (982)
- LD-14 (connection management — writer task + reader pool, size 4): `architecture.md:404`; LD-16 (Tokio, `spawn_blocking`): `architecture.md:408`; LD-4 (locked PRAGMAs via `open()`): `architecture.md:66`; LD-7 (Single Writer Rule): `architecture.md:69`; LD-37 canonical-version invariant (tokio/rusqlite never skipped): `architecture.md:1168`, `deny.toml:127-131`; FR-17: `architecture.md:1060`; NFR-3 (agenda <100ms): `epics.md:98`; NFR-4 (search <200ms): `epics.md:100`
- Predecessor seams this consumes (do **not** edit): `crates/orgsidian-index/src/connection.rs` (`open` = LD-4 PRAGMAs; `apply_schema`), `crates/orgsidian-index/src/migrations.rs` (`migrate` = LD-12, the sole migration site), `crates/orgsidian-index/src/lib.rs` (`SCHEMA_SQL`, re-exports), `crates/orgsidian-index/src/error.rs` (`IndexError`), `crates/orgsidian-index/tests/schema.rs` + `tests/migrations.rs` (must pass unchanged)
- Predecessor story (the reader-pool-must-not-migrate contract): `_bmad-output/implementation-artifacts/3-4-wire-rusqlite-migration-forward-only-migrations.md` — Dev Note 5 (`migrate` deliberately not folded into `open()`; readers must not migrate), Dev Note 2 (the wrapper↔rusqlite lock-step lesson this story reapplies)
- Deferred-work rows this story resolves/updates: the two **`busy_timeout` unset** rows (Story 3.3 + Story 3.4-review) — resolve; the `application_id`/foreign-file guard row — unchanged (still 3.6)
- Conventions to mirror: `crates/orgsidian-index/src/error.rs`, `crates/orgsidian-vault/src/error.rs`, `architecture.md:694-699` (naming), `:723-729` (test placement), `:738-742` (crate org), `:850-877` (AI-agent rules)
- Supply chain: `deny.toml:114` (`multiple-versions = "deny"`), `:127-131` (LD-37 canonical-version invariant), `:193-195` (LEAF wrappers), `docs/security/advisory-exceptions.md`, `scripts/check-allowlist-sync.mjs`, CI gates in `.github/workflows/pr.yml`
- `deadpool` docs: https://docs.rs/deadpool/latest/ (`managed::Manager`, `managed::Pool`, `Runtime`, `Status`, `PoolError`, `BuildError`)

### Project Context Reference

No `project-context.md` exists in the repo at story creation (checked 2026-08-13). `architecture.md`, the epic ACs, Story 3.4's file, the current `crates/orgsidian-index/` source, and this story are the authoritative context.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8 (bmad-dev-story workflow)

### Debug Log References

- **Dependency delta (AC1):** `cargo build --workspace --locked` added exactly `deadpool 0.13.0`, `deadpool-runtime 0.3.1`, `num_cpus 1.17.0` to `Cargo.lock` — the predicted set. `num_cpus` was a genuinely new crate (not previously in the tree). `cargo tree -p orgsidian-index -i tokio` shows exactly **one** `tokio 1.52.3` (unified with Tauri's existing lock entry; `deadpool` → `tokio`, `deadpool-runtime` → `tokio`, and the crate's own dep all resolve to it). `cargo tree -p orgsidian-index -i rusqlite` shows exactly **one** `rusqlite 0.40.1` (via the crate + `rusqlite_migration 2.6.0`); `deadpool` has **no** `rusqlite` edge, so the 0.40 pin is untouched. `cargo deny check bans` → **bans ok** (no duplicate major fired), so `deny.toml` stays untouched.
- **Anti-placebo mutation (AC4):** temporarily set `BUSY_TIMEOUT` to `Duration::from_secs(0)` and re-ran `pooled_connections_carry_the_configured_busy_timeout` — it **failed** as required (`assertion left == right failed … left: 0, right: 5000`). Restored to `5s`; all 4 concurrency tests green again.
- **`cargo audit`:** bare `cargo audit` reports 1 vulnerability — `RUSTSEC-2026-0235` (rkyv) — but this is **pre-existing and unrelated to this story**: `rkyv` count is identical (1) in `main`'s lock and this branch's lock, it is not in the `orgsidian-index` tree (not reachable from `deadpool`/`tokio`), and it is already in the ledger (`docs/security/advisory-exceptions.md` + `.cargo/audit-ignore.txt`). CI's gated invocation (`cargo audit --deny warnings $IGNORES` from `.cargo/audit-ignore.txt`) exits **0**. Nothing new fired, so no ledger edit was made (staying inside the AC5 scope fence).

### Completion Notes List

- **AC1 — dependency delta.** Root `Cargo.toml` gains generic `deadpool = { version = "0.13", default-features = false, features = ["managed", "rt_tokio_1"] }` and `tokio = { version = "1", default-features = false }`, each with a story-attributed comment block matching the house style (LD trace, version rationale, LD-37 canonical-version coupling for `tokio`, license). Crate `Cargo.toml` consumes `deadpool = { workspace = true }`, `tokio = { workspace = true, features = ["sync", "rt"] }`, and dev-`tokio` `["macros", "rt-multi-thread"]`. Single `tokio`, single `rusqlite`, `deny bans` clean — see Debug Log.
- **Disclosed deviation — LD-14 `deadpool-sqlite` → generic `deadpool`.** Built as the maintainer-confirmed generic-`deadpool` + local `Manager` design (Dev Note 1), not `deadpool-sqlite` (which pins `rusqlite ^0.38` and would break LD-37). Filed as a `deferred-work.md` architecture-addendum row; `architecture.md` archive **not** edited in place (same handling as the Story 3.3 LD-41 and Story 3.4 LD-11 wording deviations).
- **AC2 — `src/pool.rs`.** `ConnectionManager` implements `deadpool::managed::Manager` (`type Type = rusqlite::Connection`, `type Error = IndexError`): `create` = `open()` (LD-4 PRAGMAs, reused verbatim) + `busy_timeout`, wrapped in `spawn_blocking` (LD-16); `recycle` = a `SELECT 1` liveness probe (documented as the one rusqlite call exempt from `spawn_blocking` — constant expression, no disk I/O, and `recycle`'s borrowed `&mut Connection` cannot move into a `'static` task). `IndexPool::new` builds `Pool::builder(…).max_size(DEFAULT_READERS=4).runtime(Runtime::Tokio1).build()`, mapping `BuildError` → `IndexError::PoolBuild`. `IndexPool::interact<F,R>` acquires a pooled connection (mapping `PoolError` → `IndexError::PoolAcquire`) and runs the closure in `spawn_blocking`. `const BUSY_TIMEOUT = 5s` with documented rationale. No domain SQL. `IndexPool::status()` added so the test can assert connections were returned.
- **AC3 — `src/writer.rs`.** `IndexWriter::spawn` opens the one writable connection, sets `busy_timeout`, and runs `migrate` **once** inline (so a migration failure surfaces synchronously from `spawn`) — the sole migration site — then spawns the drain loop on `spawn_blocking`, owning `conn` and draining a **bounded** `mpsc` via `blocking_recv`. `IndexUpdate` = boxed `FnOnce(&mut Connection) -> Result<(), IndexError>` thunk + `oneshot` ack (transport only, Dev Note 4). `IndexWriter::execute` boxes/sends/awaits; send failure or a dropped ack → `IndexError::WriterUnavailable` (distinct from a SQL error). Graceful shutdown: dropping the writer closes the channel (`blocking_recv` → `None`); `shutdown(self)` is the explicit drain-and-await form.
- **Ordering footgun (Dev Note 5).** No `Index` facade was added — AC5's file list does not permit a new `index.rs`, and Dev Note 8 forbids logic in `lib.rs`. Took the documented fallback instead: loud "migrate before you read" ordering docs on both `IndexPool` and `IndexWriter`, and the crate-doc, so Story 3.6 cannot reads-before-migrate by accident.
- **AC4 — `tests/concurrency.rs`.** Real on-disk `TempDir` DB per test, `#[tokio::test(flavor = "multi_thread")]`. Four tests: (1) 16 concurrent reads on 4 readers → all `Ok`, then `status().available == 4` (nothing leaked); (2) 32 concurrent read-modify-write increments through the writer → sum is exactly 32, no `SQLITE_BUSY` (serialization, no lost updates); (3) reads interleaved with writes → no deadlock + read-after-write observes committed state; (4) `PRAGMA busy_timeout` reads back 5000 on both a pooled and the writer connection. Anti-placebo mutation recorded in Debug Log.
- **AC5 — scope + gates.** `git diff main --name-only` shows only the AC5-permitted files. Sentinels byte-untouched (`connection.rs`, `migrations.rs`, `tests/schema.rs`, `tests/migrations.rs`, `migrations/0001_initial-schema.sql`, `architecture.md`, `deny.toml`, all `anchor.rs`, `orgsidian-vault/`/`-parser/`/`-watcher/`, `.github/workflows/`). `EXPECTED_REMAINING_PLACEHOLDERS` unchanged (failure-modes files untouched). Both `busy_timeout` deferred rows marked RESOLVED; four new deferred rows filed (LD-14 wording, `deadpool`↔`rusqlite` re-check coupling, read-only-reader hardening, writer channel-bound calibration). All gates green: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked` (all suites pass), `cargo deny --locked check all` (advisories/bans/licenses/sources ok), CI-gated `cargo audit` (exit 0).

### File List

- `Cargo.toml` (modified — root `[workspace.dependencies]`: `deadpool`, `tokio`)
- `Cargo.lock` (modified — +`deadpool 0.13.0`, +`deadpool-runtime 0.3.1`, +`num_cpus 1.17.0`)
- `crates/orgsidian-index/Cargo.toml` (modified — `deadpool` + `tokio` deps + dev-`tokio`)
- `crates/orgsidian-index/src/pool.rs` (new — `ConnectionManager`, `IndexPool`, `BUSY_TIMEOUT`)
- `crates/orgsidian-index/src/writer.rs` (new — `IndexWriter`, `IndexUpdate`)
- `crates/orgsidian-index/src/error.rs` (modified — `PoolBuild`, `PoolAcquire`, `WriterUnavailable` variants)
- `crates/orgsidian-index/src/lib.rs` (modified — `pool`/`writer` module decls + re-exports + crate-doc sentence)
- `crates/orgsidian-index/tests/concurrency.rs` (new — 4 concurrency tests)
- `_bmad-output/implementation-artifacts/deferred-work.md` (modified — 2 rows RESOLVED, 4 new rows)
- `_bmad-output/implementation-artifacts/3-5-add-deadpool-sqlite-reader-pool-dedicated-writer-task.md` (this story — Dev Agent Record, tasks, status)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified — story 3.5 → review)

## Change Log

- 2026-08-13 — Story created (bmad-create-story). Decision: pivot from `deadpool-sqlite` (pins `rusqlite ^0.38`, conflicts with the workspace's `rusqlite 0.40` and LD-37) to **generic `deadpool` + a local `Manager`** over the rusqlite-0.40 `Connection` — maintainer-confirmed. Resolves the two `busy_timeout` deferred-work rows assigned to Story 3.5. Status → ready-for-dev.
- 2026-08-13 — Story implemented (bmad-dev-story). Added `src/pool.rs` (generic-`deadpool` reader pool of 4, `BUSY_TIMEOUT`, `interact` helper), `src/writer.rs` (single dedicated writer task, bounded `mpsc`, `IndexUpdate` transport), `error.rs` pool/writer variants, `lib.rs` re-exports, and `tests/concurrency.rs` (16-read / 32-write / interleaved / `busy_timeout` read-back, anti-placebo verified). Single `tokio`/single `rusqlite` confirmed; `deny bans` clean (no `deny.toml` change). Both `busy_timeout` deferred rows RESOLVED; 4 new deferred rows filed. All gates green. Status → review.
