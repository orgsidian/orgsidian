//! Test-support surface (gated behind `cfg(any(test, feature = "test-support"))`).
//!
//! Story 1.9 ships the `clock` submodule (deterministic `Clock` trait + `FakeClock`
//! per LD-9). Story 1.12 adds the `perf` submodule alongside it.

pub mod clock;
pub mod perf;
