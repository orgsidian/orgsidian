//! Story 2.3 — semantic-layer construct tests (FR-1, LD-44).
//!
//! One `semantic_*` test per enumerated LD-44 construct (epics.md:793-807;
//! the epic's "14 constructs" label vs its 15 enumerated bullets resolves to
//! the bullets — all 15 are covered here; `semantic_{construct_kebab}` names
//! resolve to snake_case because kebab-case is not a valid Rust identifier).
//!
//! Constructs the semantic layer does not model (inline markup, lists,
//! tables, blocks, inline LaTeX, footnotes, citations) get non-crash
//! regression tests that still pin a real property (headline structure
//! intact + construct text present in the headline's span) — no placebo
//! greens per the Story 1.9 discipline. The documented gaps live in
//! `docs/parser/KNOWN_DIVERGENCES.md`.

use chrono::{NaiveDate, NaiveTime, TimeDelta};
use orgsidian_parser::semantic::{DrawerKind, LinkKind, RepeaterKind, TimeUnit, TodoConfig};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("test dates are valid")
}

fn time(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("test times are valid")
}

// 1. Heading levels 1-6 with TODO/NEXT/DONE/WAITING + custom via #+TODO:.
#[test]
fn semantic_heading_levels_with_todo_states() {
    let src = "* TODO One\n** NEXT Two\n*** DONE Three\n**** WAITING Four\n\
               ***** TODO Five\n****** DONE Six\n";
    let doc = orgsidian_parser::analyze(src).expect("nested headings must analyze");
    // Walk the nesting chain: each level nests under the previous one.
    let expected = [
        (1u8, "TODO", false, "One"),
        (2, "NEXT", false, "Two"),
        (3, "DONE", true, "Three"),
        (4, "WAITING", true, "Four"),
        (5, "TODO", false, "Five"),
        (6, "DONE", true, "Six"),
    ];
    let mut current = &doc.headlines;
    for (level, keyword, done, title) in expected {
        assert_eq!(current.len(), 1, "exactly one headline at level {level}");
        let h = &current[0];
        assert_eq!(h.level, level);
        let todo = h.todo_state.as_ref().expect("keyword must be recognized");
        assert_eq!(todo.keyword, keyword);
        assert_eq!(todo.done, done, "{keyword} done-class");
        assert_eq!(h.title, title);
        current = &h.children;
    }

    // Default cycling order TODO → NEXT → DONE → WAITING → TODO (wrap).
    let cfg = TodoConfig::default();
    assert_eq!(cfg.next(None), Some("TODO"));
    assert_eq!(cfg.next(Some("TODO")), Some("NEXT"));
    assert_eq!(cfg.next(Some("NEXT")), Some("DONE"));
    assert_eq!(cfg.next(Some("DONE")), Some("WAITING"));
    assert_eq!(cfg.next(Some("WAITING")), Some("TODO"), "must wrap");

    // An in-file #+TODO: REPLACES the default set for that document.
    let custom = "#+TODO: DRAFT | PUBLISHED\n* DRAFT Post\n* TODO Plain words\n";
    let doc = orgsidian_parser::analyze(custom).expect("custom TODO doc must analyze");
    let h0 = &doc.headlines[0];
    let todo = h0.todo_state.as_ref().expect("DRAFT must be recognized");
    assert_eq!(todo.keyword, "DRAFT");
    assert!(!todo.done);
    assert_eq!(h0.title, "Post");
    // `TODO` is no longer a configured keyword — it is title text now.
    let h1 = &doc.headlines[1];
    assert!(
        h1.todo_state.is_none(),
        "unconfigured keyword is title text"
    );
    assert_eq!(h1.title, "TODO Plain words");
    // Cycling follows the directive's declaration order and wraps.
    assert_eq!(doc.todo_config.next(None), Some("DRAFT"));
    assert_eq!(doc.todo_config.next(Some("DRAFT")), Some("PUBLISHED"));
    assert_eq!(doc.todo_config.next(Some("PUBLISHED")), Some("DRAFT"));
    assert_eq!(doc.todo_config.next(Some("TODO")), None, "unknown keyword");
}

// 2. SCHEDULED: timestamps, active <…> and inactive […].
#[test]
fn semantic_scheduled_timestamp() {
    let src = "* Active\nSCHEDULED: <2026-06-10 Wed 10:00>\n\
               * Inactive\nSCHEDULED: [2026-06-11 Thu]\n";
    let doc = orgsidian_parser::analyze(src).expect("scheduled sample must analyze");
    let active = doc.headlines[0].scheduled.as_ref().expect("scheduled set");
    assert!(active.active, "<…> is active");
    assert_eq!(active.date, date(2026, 6, 10));
    assert_eq!(active.time, Some(time(10, 0)));
    assert!(doc.headlines[0].deadline.is_none());

    let inactive = doc.headlines[1].scheduled.as_ref().expect("scheduled set");
    assert!(!inactive.active, "[…] is inactive");
    assert_eq!(inactive.date, date(2026, 6, 11));
    assert_eq!(inactive.time, None);
    // Spans index the original source: the raw text is recoverable.
    assert_eq!(&src[active.span.clone()], "<2026-06-10 Wed 10:00>");
    assert_eq!(&src[inactive.span.clone()], "[2026-06-11 Thu]");
}

// 3. DEADLINE: timestamps, active + inactive.
#[test]
fn semantic_deadline_timestamp() {
    let src = "* Active\nDEADLINE: <2026-06-12 Fri 09:30>\n\
               * Inactive\nDEADLINE: [2026-06-13 Sat]\n";
    let doc = orgsidian_parser::analyze(src).expect("deadline sample must analyze");
    let active = doc.headlines[0].deadline.as_ref().expect("deadline set");
    assert!(active.active);
    assert_eq!(active.date, date(2026, 6, 12));
    assert_eq!(active.time, Some(time(9, 30)));
    assert!(doc.headlines[0].scheduled.is_none());

    let inactive = doc.headlines[1].deadline.as_ref().expect("deadline set");
    assert!(!inactive.active);
    assert_eq!(inactive.date, date(2026, 6, 13));
    assert_eq!(inactive.time, None);
}

// 4. CLOCK entries inside :LOGBOOK: — closed (ranged + duration) and open.
#[test]
fn semantic_clock_entries() {
    let src = "* Clocked\n:LOGBOOK:\n\
               CLOCK: [2026-06-09 Tue 10:00]--[2026-06-09 Tue 11:30] =>  1:30\n\
               CLOCK: [2026-06-10 Wed 08:00]\n\
               :END:\n";
    let doc = orgsidian_parser::analyze(src).expect("clock sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.clocks.len(), 2, "both CLOCK lines surface from :LOGBOOK:");

    let closed = &h.clocks[0];
    assert_eq!(closed.start.date, date(2026, 6, 9));
    assert_eq!(closed.start.time, Some(time(10, 0)));
    assert!(!closed.start.active, "CLOCK timestamps are inactive");
    let end = closed.end.as_ref().expect("closed entry has an end");
    assert_eq!(end.date, date(2026, 6, 9));
    assert_eq!(end.time, Some(time(11, 30)));
    assert_eq!(closed.duration, TimeDelta::try_minutes(90));

    let open = &h.clocks[1];
    assert_eq!(open.start.date, date(2026, 6, 10));
    assert_eq!(open.start.time, Some(time(8, 0)));
    assert!(open.end.is_none(), "open entry has no end");
    assert!(open.duration.is_none(), "open entry has no duration");
}

// 5. Recurring timestamps: repeater kind / value / unit on plan-position stamps.
#[test]
fn semantic_recurring_timestamps() {
    // The epic's enumerated literals (`+1w`, `+1d`, `+1m`, `+1y`) verbatim,
    // plus the `++`/`.+` kinds (review fix: the cumulate month/year forms
    // were previously substituted, not covered).
    let src = "* W\nSCHEDULED: <2026-06-10 Wed +1w>\n\
               * D\nSCHEDULED: <2026-06-10 Wed +1d>\n\
               * M\nSCHEDULED: <2026-06-10 Wed +1m>\n\
               * Y\nSCHEDULED: <2026-06-10 Wed +1y>\n\
               * CU\nSCHEDULED: <2026-06-10 Wed ++1m>\n\
               * RS\nSCHEDULED: <2026-06-10 Wed .+1y>\n";
    let doc = orgsidian_parser::analyze(src).expect("recurring sample must analyze");
    let expected = [
        (RepeaterKind::Cumulate, 1u32, TimeUnit::Week),
        (RepeaterKind::Cumulate, 1, TimeUnit::Day),
        (RepeaterKind::Cumulate, 1, TimeUnit::Month),
        (RepeaterKind::Cumulate, 1, TimeUnit::Year),
        (RepeaterKind::CatchUp, 1, TimeUnit::Month),
        (RepeaterKind::Restart, 1, TimeUnit::Year),
    ];
    assert_eq!(doc.headlines.len(), expected.len());
    for (h, (kind, value, unit)) in doc.headlines.iter().zip(expected) {
        let ts = h.scheduled.as_ref().expect("scheduled set");
        let rep = ts.repeater.as_ref().expect("repeater parsed");
        assert_eq!(rep.kind, kind, "headline {:?}", h.title);
        assert_eq!(rep.value, value);
        assert_eq!(rep.unit, unit);
    }
}

// 6. Drawer types: :PROPERTIES:, :LOGBOOK:, custom.
#[test]
fn semantic_drawer_types() {
    let src = "* H\n:PROPERTIES:\n:ID: abc-123\n:CUSTOM_ID: xyz\n:END:\n\
               :LOGBOOK:\nCLOCK: [2026-06-10 Wed 08:00]\n:END:\n\
               :MYDRAWER:\nfree text\n:END:\n";
    let doc = orgsidian_parser::analyze(src).expect("drawer sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.properties.len(), 2);
    assert_eq!(h.properties.get("ID").map(String::as_str), Some("abc-123"));
    assert_eq!(
        h.properties.get("CUSTOM_ID").map(String::as_str),
        Some("xyz")
    );

    let kinds: Vec<&DrawerKind> = h.drawers.iter().map(|d| &d.kind).collect();
    assert!(kinds.contains(&&DrawerKind::Properties), "{kinds:?}");
    assert!(kinds.contains(&&DrawerKind::Logbook), "{kinds:?}");
    assert!(
        kinds.contains(&&DrawerKind::Custom("MYDRAWER".to_string())),
        "{kinds:?}"
    );
    let custom = h
        .drawers
        .iter()
        .find(|d| matches!(d.kind, DrawerKind::Custom(_)))
        .expect("custom drawer present");
    assert!(custom.contents.contains("free text"));
    assert_eq!(&src[custom.contents_span.clone()], custom.contents);
}

// 7. Inline markup tolerated (NOT semantically modeled — documented gap).
#[test]
fn semantic_inline_markup() {
    let src = "* Markup heading\n\
               Text with *bold* /italic/ =verbatim= ~code~ +strike+ _underline_ words.\n";
    let doc = orgsidian_parser::analyze(src).expect("markup sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.level, 1);
    assert_eq!(h.title, "Markup heading");
    // The gap is real: markup survives as raw text inside the headline span.
    let section = &src[h.span.clone()];
    for marker in [
        "*bold*",
        "/italic/",
        "=verbatim=",
        "~code~",
        "+strike+",
        "_underline_",
    ] {
        assert!(
            section.contains(marker),
            "{marker} must survive as raw text"
        );
    }
}

// 8. Link types: id / wiki (± description) / file / bracketed url / plain url.
#[test]
fn semantic_links() {
    let src = "Preamble has [[id:abc]] link.\n\
               * Links\n\
               See [[wiki page][the docs]], [[file://notes/x.org]], \
               [[https://example.com/page][site]], [[other]] and plain \
               http://example.com here.\n";
    let doc = orgsidian_parser::analyze(src).expect("links sample must analyze");

    let preamble = doc.preamble.as_ref().expect("preamble present");
    assert_eq!(preamble.links.len(), 1);
    let id = &preamble.links[0];
    assert_eq!(id.kind, LinkKind::Id);
    assert_eq!(id.target, "id:abc");
    assert!(id.description.is_none());
    assert_eq!(&src[id.span.clone()], "[[id:abc]]");

    let links = &doc.headlines[0].links;
    assert_eq!(links.len(), 5, "{links:?}");

    assert_eq!(links[0].kind, LinkKind::Wiki);
    assert_eq!(links[0].target, "wiki page");
    assert_eq!(links[0].description.as_deref(), Some("the docs"));

    assert_eq!(links[1].kind, LinkKind::File);
    assert_eq!(links[1].target, "file://notes/x.org");
    assert!(links[1].description.is_none());

    assert_eq!(links[2].kind, LinkKind::Url);
    assert_eq!(links[2].target, "https://example.com/page");
    assert_eq!(links[2].description.as_deref(), Some("site"));

    assert_eq!(links[3].kind, LinkKind::Wiki);
    assert_eq!(links[3].target, "other");
    assert!(links[3].description.is_none());

    assert_eq!(links[4].kind, LinkKind::Plain);
    assert_eq!(links[4].target, "http://example.com");
    assert_eq!(&src[links[4].span.clone()], "http://example.com");
}

// 9. Lists tolerated (outside the semantic surface; checkbox semantics a gap).
#[test]
fn semantic_lists() {
    let src = "* List heading\n- one\n+ two\n1. three\n- [ ] open task\n- [X] done task\n";
    // No ERROR nodes: lists are grammar-modeled, just not semantically lifted.
    let tree = orgsidian_parser::parse(src).expect("lists must parse");
    assert!(!tree.root_node().has_error(), "no ERROR nodes for lists");
    let doc = orgsidian_parser::analyze(src).expect("lists sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.title, "List heading");
    let section = &src[h.span.clone()];
    assert!(
        section.contains("- [ ] open task"),
        "checkbox raw text intact"
    );
    assert!(section.contains("- [X] done task"));
}

// 10. Tables tolerated (incl. separator row and #+TBLFM: formula line).
#[test]
fn semantic_tables() {
    let src = "* Table heading\n| a | b |\n|---+---|\n| 1 | 2 |\n#+TBLFM: $2=$1*2\n";
    let tree = orgsidian_parser::parse(src).expect("tables must parse");
    assert!(!tree.root_node().has_error(), "no ERROR nodes for tables");
    let doc = orgsidian_parser::analyze(src).expect("table sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.title, "Table heading");
    let section = &src[h.span.clone()];
    assert!(section.contains("#+TBLFM: $2=$1*2"), "formula line in span");
    assert!(section.contains("|---+---|"), "separator row in span");
}

// 11. Block elements tolerated: SRC / QUOTE / EXAMPLE / VERSE.
#[test]
fn semantic_block_elements() {
    let src = "* Blocks\n\
               #+BEGIN_SRC rust\nfn main() {}\n#+END_SRC\n\
               #+BEGIN_QUOTE\nq\n#+END_QUOTE\n\
               #+BEGIN_EXAMPLE\ne\n#+END_EXAMPLE\n\
               #+BEGIN_VERSE\nv\n#+END_VERSE\n";
    let tree = orgsidian_parser::parse(src).expect("blocks must parse");
    assert!(!tree.root_node().has_error(), "no ERROR nodes for blocks");
    let doc = orgsidian_parser::analyze(src).expect("blocks sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.title, "Blocks");
    let section = &src[h.span.clone()];
    for begin in [
        "#+BEGIN_SRC",
        "#+BEGIN_QUOTE",
        "#+BEGIN_EXAMPLE",
        "#+BEGIN_VERSE",
    ] {
        assert!(section.contains(begin), "{begin} in span");
    }
}

// 12. Inline LaTeX tolerated (NOT grammar-modeled — documented gap).
#[test]
fn semantic_inline_latex() {
    let src = "* Latex\nInline $x^2$ and \\(a+b\\) and \\[c\\] forms.\n";
    let doc = orgsidian_parser::analyze(src).expect("latex sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.title, "Latex");
    let section = &src[h.span.clone()];
    for marker in ["$x^2$", "\\(a+b\\)", "\\[c\\]"] {
        assert!(section.contains(marker), "{marker} survives as raw text");
    }
}

// 13. Footnotes tolerated: line-start [fn:N] definition + inline [fn::…] ref.
#[test]
fn semantic_footnotes() {
    let src = "* Notes\nText with an inline[fn::inline note] ref and a normal[fn:1] ref.\n\n\
               [fn:1] The definition line.\n";
    let doc = orgsidian_parser::analyze(src).expect("footnote sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.title, "Notes");
    let section = &src[h.span.clone()];
    assert!(section.contains("[fn::inline note]"), "inline ref intact");
    assert!(
        section.contains("[fn:1] The definition line."),
        "fndef intact"
    );
    // Single-bracket forms must NOT be misread as links by the scanner.
    assert!(
        h.links.is_empty(),
        "footnote brackets are not links: {:?}",
        h.links
    );
}

// 14. Citations tolerated (NOT grammar-modeled — documented gap).
#[test]
fn semantic_citations() {
    let src = "* Cited\nClaim backed by [cite:@key2026] here.\n";
    let doc = orgsidian_parser::analyze(src).expect("citation sample must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.title, "Cited");
    let section = &src[h.span.clone()];
    assert!(
        section.contains("[cite:@key2026]"),
        "citation raw text intact"
    );
    assert!(
        h.links.is_empty(),
        "citation brackets are not links: {:?}",
        h.links
    );
}

// 15. Tags: single and multiple, names without colons.
#[test]
fn semantic_tags() {
    let src = "* One :tag:\n* Two :tag1:tag2:\n";
    let doc = orgsidian_parser::analyze(src).expect("tags sample must analyze");
    let single: Vec<&str> = doc.headlines[0]
        .tags
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(single, ["tag"]);
    let multi: Vec<&str> = doc.headlines[1]
        .tags
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(multi, ["tag1", "tag2"]);
    // Tag spans point at the bare name inside the source.
    let t0 = &doc.headlines[0].tags[0];
    assert_eq!(&src[t0.span.clone()], "tag");
}

// --- Beyond the 15 LD-44 bullets: lenience + config edge regression tests ---

// AC5: unparseable date inside a well-shaped timestamp must not abort analyze().
#[test]
fn semantic_lenient_unparseable_date() {
    let src = "* H\nSCHEDULED: <2026-13-40 Xxx>\n";
    let doc = orgsidian_parser::analyze(src).expect("bad date must not abort analyze");
    let h = &doc.headlines[0];
    assert!(h.scheduled.is_none(), "month 13 is skipped, not crashed on");
    assert_eq!(h.title, "H", "headline structure intact");
}

// AC2: #+TODO: without a pipe — the LAST keyword is the done set.
#[test]
fn semantic_todo_directive_without_pipe() {
    let src = "#+TODO: ONE TWO THREE\n* THREE x\n* ONE y\n";
    let doc = orgsidian_parser::analyze(src).expect("no-pipe directive must analyze");
    let done = doc.headlines[0]
        .todo_state
        .as_ref()
        .expect("THREE recognized");
    assert!(done.done, "last keyword without pipe is the done class");
    let active = doc.headlines[1]
        .todo_state
        .as_ref()
        .expect("ONE recognized");
    assert!(!active.done);
    assert_eq!(doc.todo_config.next(Some("THREE")), Some("ONE"), "wraps");
}

// AC2: multiple #+TODO: lines accumulate; cycling stays within a sequence.
#[test]
fn semantic_multiple_todo_directives_accumulate() {
    let src = "#+TODO: A B | C\n#+SEQ_TODO: X | Y\n* X q\n* B r\n";
    let doc = orgsidian_parser::analyze(src).expect("multi-directive must analyze");
    assert!(
        doc.headlines[0].todo_state.is_some(),
        "2nd sequence recognized"
    );
    assert!(
        doc.headlines[1].todo_state.is_some(),
        "1st sequence recognized"
    );
    // Cycling wraps within the keyword's own sequence, org-style.
    assert_eq!(doc.todo_config.next(Some("C")), Some("A"));
    assert_eq!(doc.todo_config.next(Some("X")), Some("Y"));
    assert_eq!(doc.todo_config.next(Some("Y")), Some("X"));
}

// AC1: duplicate property keys collapse last-wins (documented HashMap caveat).
#[test]
fn semantic_duplicate_property_keys_last_wins() {
    let src = "* H\n:PROPERTIES:\n:KEY: first\n:KEY: second\n:END:\n";
    let doc = orgsidian_parser::analyze(src).expect("dup keys must analyze");
    let h = &doc.headlines[0];
    assert_eq!(h.properties.len(), 1);
    assert_eq!(h.properties.get("KEY").map(String::as_str), Some("second"));
}

// AC1: document preamble carries text + span + directives; empty doc is fine.
#[test]
fn semantic_preamble_and_empty_document() {
    let src = "#+TITLE: My doc\nIntro paragraph.\n* First\n";
    let doc = orgsidian_parser::analyze(src).expect("preamble doc must analyze");
    let pre = doc.preamble.as_ref().expect("preamble present");
    // Review fix: assert against the expected literal (the old
    // `src[span] == text` check was true by construction).
    assert_eq!(pre.text, "#+TITLE: My doc\nIntro paragraph.\n");
    assert_eq!(pre.span, 0..pre.text.len());
    assert!(
        pre.directives
            .iter()
            .any(|d| d.name == "TITLE" && d.value == "My doc"),
        "{:?}",
        pre.directives
    );

    let empty = orgsidian_parser::analyze("").expect("empty source analyzes");
    assert!(empty.headlines.is_empty());
    assert!(empty.preamble.is_none());
    assert_eq!(empty.todo_config, TodoConfig::default());
}

// AC5/AC1 (review fix): CLOSED: plan entries route into `Headline::closed`.
#[test]
fn semantic_closed_timestamp() {
    let src = "* DONE Shipped\nCLOSED: [2026-06-09 Tue 18:15] DEADLINE: <2026-06-12 Fri>\n";
    let doc = orgsidian_parser::analyze(src).expect("closed sample must analyze");
    let h = &doc.headlines[0];
    let closed = h.closed.as_ref().expect("closed set");
    assert!(!closed.active, "org writes CLOSED stamps inactive");
    assert_eq!(closed.date, date(2026, 6, 9));
    assert_eq!(closed.time, Some(time(18, 15)));
    assert!(h.deadline.is_some(), "same plan line still routes DEADLINE");
    assert!(h.scheduled.is_none());
}

// AC2 (review fix): directive NAMES match case-insensitively (org-style) —
// `#+todo:` configures the document exactly like `#+TODO:`.
#[test]
fn semantic_lowercase_todo_directive_name() {
    let src = "#+todo: DRAFT | FINAL\n* DRAFT x\n";
    let doc = orgsidian_parser::analyze(src).expect("lowercase directive must analyze");
    let todo = doc.headlines[0]
        .todo_state
        .as_ref()
        .expect("DRAFT recognized via #+todo:");
    assert!(!todo.done);
    assert_eq!(doc.todo_config.next(Some("DRAFT")), Some("FINAL"));
}

// AC2 regression (review): `#+TODO:` lines inside block/drawer contents are
// expr soup at the pinned grammar SHA — they must NOT feed TodoConfig.
#[test]
fn semantic_directives_inside_blocks_and_drawers_are_inert() {
    let src = "#+BEGIN_EXAMPLE\n#+TODO: HIJACK | PWNED\n#+END_EXAMPLE\n\
               * H\n:MYDRAWER:\n#+TODO: DRAWERJACK | D\n:END:\n\
               * HIJACK x\n";
    let doc = orgsidian_parser::analyze(src).expect("quoted directives must analyze");
    assert_eq!(
        doc.todo_config,
        TodoConfig::default(),
        "quoted #+TODO: lines must not replace the default config"
    );
    assert!(
        doc.headlines[1].todo_state.is_none(),
        "HIJACK is title text, not a state"
    );
}

// AC5: timestamp date ranges (`<a>--<b>`) and time ranges (`10:00-11:00`).
#[test]
fn semantic_timestamp_ranges() {
    let src = "* R\nSCHEDULED: <2026-06-10 Wed>--<2026-06-12 Fri>\n\
               * T\nSCHEDULED: <2026-06-10 Wed 10:00-11:00>\n";
    let doc = orgsidian_parser::analyze(src).expect("ranges must analyze");
    let r = doc.headlines[0].scheduled.as_ref().expect("date range set");
    assert_eq!(r.date, date(2026, 6, 10));
    assert_eq!(r.end_date, Some(date(2026, 6, 12)));
    let t = doc.headlines[1].scheduled.as_ref().expect("time range set");
    assert_eq!(t.time, Some(time(10, 0)));
    assert_eq!(t.end_time, Some(time(11, 0)));
    assert!(t.end_date.is_none());
}
