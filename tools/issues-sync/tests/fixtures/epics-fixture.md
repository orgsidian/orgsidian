# Smoke Fixture — tools/issues-sync wiremock integration test

This file is intentionally not real epic content; it's a 2-story smoke
fixture for `tools/issues-sync/tests/sync_smoke.rs` (Story 1.16 / LD-55 AC6).
The `## Epic List` section exists to exercise the parser's section-skip
state machine. `## Epic 3:` deep section yields exactly 2 stories.

Stories use `### Story 3.91:` / `### Story 3.92:` (out-of-range from real
Epic 3 stories `3.1`–`3.7`) so this fixture cannot accidentally collide
with production story numbers if it ever leaks into production parsing.
Epic 3 maps to milestone `v0.1` via `milestone_for_epic` (the `1..=6` bucket).

## Epic List

### Epic 3: Overview line — must NOT yield a Story record (parser skip-test).

## Epic 3: Smoke Fixture

### Story 3.91: First smoke story

As the **smoke tester**,
I want a parser-friendly two-story fixture,
so that the wiremock integration test can exercise the full sync pipeline.

**Acceptance Criteria:**

- This is AC1 for the smoke fixture.
- This is AC2 — a second bullet to exercise the AC block extractor.

**Traces:** LD-55 (smoke surface), LD-37 (test-strategy)

### Story 3.92: Second smoke story

As a **fixture author**,
I want a second story with the same shape as the first,
so that the wiremock integration test asserts `Times::Exactly(2)` on `POST /issues`.

**Acceptance Criteria:**

- Single-bullet AC for brevity.

**Traces:** LD-55

## End

Trailing h2 forces the parser to flush Story 3.92 cleanly.
