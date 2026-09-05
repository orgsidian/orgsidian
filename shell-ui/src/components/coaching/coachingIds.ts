// Implements FR-21 (partial) / FR-18 / UJ-4 (Story 6.6).
//
// The two hardcoded UJ-4 coaching-balloon ids. MUST match, byte-for-byte, the
// Rust constants of the same name in
// `crates/orgsidian-core/src/coaching.rs` — the wire is a plain `String`
// (no specta enum), so this is the one place the frontend re-states them.
// Story 11.4 imports these same string literals when it replaces the
// hardcoded balloons with the registry-driven `CoachingSlot` API, so existing
// dismissals in `<Vault>/.orgsidian/coaching-dismissed.json` keep working.

/** The "this is your day" balloon anchored to the first Today Agenda item. */
export const UJ4_TODAY_INTRO = "UJ4_TODAY_INTRO";

/** The Quick Capture nudge balloon (v0.1 anchor: top of the `/today` route —
 *  see the Story 6.6 story file's Design Notes). */
export const UJ4_CAPTURE_INTRO = "UJ4_CAPTURE_INTRO";
