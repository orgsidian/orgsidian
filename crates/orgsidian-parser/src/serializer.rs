//! Implements FR-2.
//!
//! Round-trip-faithful serializer (Story 2.4): emits org text back from the
//! semantic structs by **raw-region passthrough**, never by re-rendering
//! semantic fields.
//!
//! # Design contract
//!
//! - **Raw-region passthrough.** [`serialize`] concatenates each headline's
//!   retained [`Headline::raw`] own-region text followed by its children,
//!   recursively, in document order. [`serialize_document`] prepends the
//!   preamble text. No semantic field is ever re-rendered: no `properties`
//!   map iteration, no `title` reconstruction, no timestamp formatting —
//!   raw text in, raw text out.
//! - **Why field-driven rendering is forbidden:** it is structurally lossy.
//!   [`Headline::title`] is trimmed (trailing headline whitespace — an LD-44
//!   edge-bucket case — would be destroyed), [`Headline::properties`] is an
//!   unordered last-wins `HashMap` (duplicate keys, ordering, and drawer
//!   framing lost), and timestamps re-rendered from `chrono` fields would
//!   lose exact day-name text and spacing. FR-2 is byte-fidelity, not
//!   semantic re-rendering.
//! - **Tiling invariant** (established by [`crate::analyze`], documented on
//!   [`Headline::raw`]): preamble text plus the recursive concatenation of
//!   all `raw` fields reproduces the analyzed source byte-for-byte, 0..len —
//!   nothing dropped, nothing duplicated. The serializer is the trivial fold
//!   over that invariant: **infallible and pure** (`-> String`, no `Result`,
//!   no I/O).
//! - **Zero-normalization posture.** FR-2's "modulo trailing-newline
//!   normalization, documented" allowance is **not exercised**: serialization
//!   is exact, including trailing bytes (trailing blank lines live inside the
//!   last section's span at the pinned grammar SHA — verified empirically,
//!   Story 2.4 Dev Notes §1). No Settings surface is required because no
//!   normalization occurs; this module doc is the documentation of record
//!   for that posture.
//!
//! Mutation-aware serialization (edit one semantic field → emit the change)
//! is Epic 4's edit-application concern, explicitly out of scope here.

use crate::semantic::{Document, Headline};

/// Serialize headlines back to org text by raw-region passthrough.
///
/// Emits each headline's retained [`Headline::raw`] own-region text followed
/// by its [`Headline::children`], recursively, concatenated in document
/// order. For a document without a preamble this reproduces the analyzed
/// source byte-for-byte (the tiling invariant); for whole files use
/// [`serialize_document`], which also emits the preamble.
///
/// Infallible and pure: no I/O, no error path, no normalization.
pub fn serialize(headlines: &[Headline]) -> String {
    let mut out = String::new();
    for headline in headlines {
        emit(headline, &mut out);
    }
    out
}

/// Recursive raw-region emission: own region first, then children in
/// document order — the exact inverse of the `analyze()` walk.
fn emit(headline: &Headline, out: &mut String) {
    out.push_str(&headline.raw);
    for child in &headline.children {
        emit(child, out);
    }
}

/// Serialize a whole [`Document`] back to org text, byte-identical to the
/// [`crate::analyze`] input.
///
/// Emits the preamble text (the zeroth section: `#+TITLE:`, directives,
/// intro text — when present), then every headline via [`serialize`]. This
/// is the FR-2 round-trip entry point: for any input `s`,
/// `serialize_document(&analyze(s)?) == s` byte-for-byte.
///
/// Infallible and pure: no I/O, no error path, no normalization.
pub fn serialize_document(document: &Document) -> String {
    let mut out = String::with_capacity(document.span.len());
    if let Some(preamble) = &document.preamble {
        out.push_str(&preamble.text);
    }
    for headline in &document.headlines {
        emit(headline, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze;

    // Region math: the own region stops at the first child — emission order
    // is own raw, then children, recursively.
    #[test]
    fn emit_orders_own_region_before_children() {
        let src = "* A\na body\n** B\nb body\n*** C\nc leaf\n** D\n";
        let doc = analyze(src).expect("analyze");
        let a = &doc.headlines[0];
        assert_eq!(a.raw, "* A\na body\n", "own region ends at first child");
        assert_eq!(a.children[0].raw, "** B\nb body\n");
        assert_eq!(a.children[0].children[0].raw, "*** C\nc leaf\n");
        assert_eq!(a.children[1].raw, "** D\n");
        assert_eq!(serialize(&doc.headlines), src);
    }

    // A childless headline's raw covers its whole section span.
    #[test]
    fn childless_raw_covers_whole_section() {
        let src = "* Solo\nbody\n\n\n";
        let doc = analyze(src).expect("analyze");
        let h = &doc.headlines[0];
        assert!(h.children.is_empty());
        assert_eq!(h.raw, src, "trailing blank lines stay inside the section");
        assert_eq!(serialize_document(&doc), src);
    }

    // serialize_document = preamble + serialize(headlines).
    #[test]
    fn document_serialization_includes_preamble() {
        let src = "#+TITLE: t\n\nintro\n\n* One\nbody\n";
        let doc = analyze(src).expect("analyze");
        let preamble = doc.preamble.as_ref().expect("preamble present");
        assert_eq!(
            serialize_document(&doc),
            format!("{}{}", preamble.text, serialize(&doc.headlines))
        );
        assert_eq!(serialize_document(&doc), src);
    }

    // Empty input → empty output (no preamble, no headlines, no panic).
    #[test]
    fn empty_document_serializes_to_empty() {
        let doc = analyze("").expect("analyze");
        assert_eq!(serialize_document(&doc), "");
        assert_eq!(serialize(&doc.headlines), "");
    }
}
