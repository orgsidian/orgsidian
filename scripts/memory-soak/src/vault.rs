//! Throwaway synthetic vault for the soak workloads (Story 4.9).
//!
//! Writes a set of varied `.org` files into a directory so the buffer
//! open/close (`parser::analyze`), plugin re-init (`rebuild_index`), and
//! agenda-query (`index_stats`) workloads have real, org-shaped content to
//! churn. Deliberately covers the constructs most likely to stress the
//! parser + decoration paths (headings, TODO states, tags, timestamps,
//! drawers, lists, checkboxes, links, tables, source blocks).

use std::io;
use std::path::Path;

/// Generate `file_count` `.org` files under `dir`. Each file is a few KB of
/// mixed org constructs; content varies per file so the index has distinct
/// headlines/tags/links to count.
pub fn synthesize(dir: &Path, file_count: usize) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for i in 0..file_count {
        let path = dir.join(format!("note-{i:03}.org"));
        std::fs::write(&path, file_body(i))?;
    }
    Ok(())
}

/// Build one file's body, seeded by `i` so files differ.
fn file_body(i: usize) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str(&format!(
        "#+TITLE: Soak Note {i}\n#+FILETAGS: :soak:generated:\n\n"
    ));
    // A handful of headlines with TODO states, tags, scheduling, drawers, and
    // bodies exercising inline markup, lists, checkboxes, links, and blocks.
    let states = ["TODO", "NEXT", "DONE", "WAITING"];
    for h in 0..8 {
        let state = states[(i + h) % states.len()];
        out.push_str(&format!(
            "* {state} Task {h} for note {i} :work:proj{mod3}:\n",
            mod3 = (i + h) % 3
        ));
        out.push_str(&format!(
            "  SCHEDULED: <2026-08-{day:02} Mon +1w> DEADLINE: <2026-09-{day:02} Tue>\n",
            day = (h % 27) + 1
        ));
        out.push_str(":PROPERTIES:\n:ID: note-");
        out.push_str(&format!("{i:03}-{h}\n"));
        out.push_str(":EFFORT: 1:30\n:END:\n");
        out.push_str(":LOGBOOK:\n");
        out.push_str("CLOCK: [2026-08-01 Fri 09:00]--[2026-08-01 Fri 10:30] =>  1:30\n");
        out.push_str(":END:\n");
        out.push_str(
            "Body with *bold*, /italic/, =verbatim=, ~code~, +strike+ and _underline_ text.\n",
        );
        out.push_str(
            "A link to [[id:note-000-0][the first task]] and a plain [[https://example.org]].\n",
        );
        out.push_str("- [ ] a checkbox item\n- [X] a done item\n- a plain bullet\n");
        out.push_str("1. numbered one\n2. numbered two\n\n");
        out.push_str("| name | value |\n|------+-------|\n| a | 1 |\n| b | 2 |\n\n");
        out.push_str("#+BEGIN_SRC rust\nfn main() { println!(\"soak {}\", ");
        out.push_str(&format!("{h}"));
        out.push_str("); }\n#+END_SRC\n\n");
        out.push_str("#+BEGIN_QUOTE\nA quoted paragraph for the soak corpus.\n#+END_QUOTE\n\n");
    }
    out
}
