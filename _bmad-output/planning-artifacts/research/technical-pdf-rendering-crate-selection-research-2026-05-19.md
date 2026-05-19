---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
workflowType: research
lastStep: 6
research_type: technical
research_topic: 'PDF rendering crate selection for orgsidian-report (FR-14 Project Report, v0.5 Beta wow demo)'
research_goals: 'Pick one of printpdf / genpdf / typst (embedded) / weasyprint-rs / wkhtmltopdf (subprocess) and produce an architecture amendment (LD-53) closing the open gap at architecture.md:1267 and architecture.md:1332.'
user_name: Tiziano
date: 2026-05-19
web_research_enabled: true
source_verification: true
---

# Research Report: Technical — PDF Rendering Crate Selection for `orgsidian-report`

**Date:** 2026-05-19
**Author:** Tiziano
**Research Type:** Technical
**Closes:** architecture.md:1267 (Important Gap #1) and architecture.md:1332 (Areas for Future Enhancement: "PDF rendering crate selection during v0.5 Beta Project Report sprint")

---

## Research Overview

Orgsidian's PRD (§4.3 FR-14) commits to a Project Report export as the v0.5 Beta **wow demo**: a user-triggered PDF/HTML export summarizing TODO completions, clocked time, linked-note excerpts, and milestone status for a selected scope and date range, with the acceptance criterion that "generated PDFs are visually consistent and printer-friendly (no clipping, readable typography)" (prd.md:275). LD-14 (architecture.md:778) extracts PDF rendering out of `orgsidian-core` into a dedicated **`orgsidian-report`** leaf crate so that the CLI does not pay the PDF dependency compile cost. The gap at architecture.md:1267 enumerates five spike candidates — `printpdf`, `genpdf`, `typst` (embedded interpreter), `weasyprint-rs`, `wkhtmltopdf` via subprocess — and states the decision deadline is **start of v0.5 Beta sprint**, scoping FR-14 to HTML-only until then.

This document compares those five against the six criteria the brief specifies — **typography quality, Unicode/RTL support, native dependencies, binary size, HTML/CSS template rendering capability (or primitive layout-building capability), and ecosystem health 2026** — and produces a single binding recommendation plus a ready-to-paste **LD-53** amendment for `architecture.md`.

---

## Technical Research Scope Confirmation

**Research Topic:** PDF rendering crate selection for `orgsidian-report`
**Research Goals:** Close the open gap at architecture.md:1267 (five candidates listed but not chosen) and architecture.md:1332 (spike deferred to v0.5 Beta sprint) with a single binding decision and a copy-pasteable LD-53 amendment.

**Technical Research Scope:**

- Architecture analysis — fit with `orgsidian-report` as a LEAF crate (architecture.md:913), compile-time cost isolation per LD-14 (architecture.md:778), and dependency-graph hygiene per LD-37 (architecture.md:1055).
- Implementation approaches — programmatic Rust API vs. external DSL (.typ) vs. external subprocess (Python/CLI binary); template authoring story for OQ-6 (prd.md:545 — v1.0 customization commitment).
- Technology stack — pure-Rust vs. native deps (Pango/cairo/Python/Qt), Tauri 2.x packaging implications (LD-2), MIT license compatibility (LD-1).
- Integration patterns — single in-process compile (FR-10 Quick Capture <1s envelope reuse), data binding (Rust structs from `orgsidian-core` → template variables).
- Typography & Unicode — Latin/Cyrillic/CJK/RTL coverage with first-class bidi shaping vs. ad-hoc font handling.
- Maintenance health — upstream activity 2025/2026, license, archive/EOL status, CVE posture.

**Methodology:** docs fetched via `ctx7` against the five upstream sources (`/fschutt/printpdf`, `/websites/rs_genpdfi_0_2_7`, `/typst/typst`, `/kozea/weasyprint`, `/websites/rubydoc_info_gems_wicked_pdf` as wkhtmltopdf surrogate); crate versions and download counts verified via crates.io API on 2026-05-19; upstream archive/EOL status verified via web research agent (GitHub repo state + downstream-acknowledgment threads); license compatibility verified against LD-1 (MIT).

**Scope Confirmed:** 2026-05-19

---

## Step 2 — Technology Stack & Candidate Overview

### Candidate matrix (verified 2026-05-19)

| Library | Latest release | Date | License | Language | Pure-Rust embed | HTML/CSS render | Bidi/RTL first-class | Status |
|---|---|---|---|---|---|---|---|---|
| `printpdf` | `0.9.1` | 2026-02-17 | MIT | Rust | ✅ | ✅ (feature `html`, experimental) | ⚠️ Unicode yes, bidi not advertised | Active (1.25M downloads) |
| `genpdf` | `0.2.0` | 2021-06-17 | Apache-2.0 | Rust | ✅ | ❌ | ❌ | **Abandoned** (last release 2021) |
| `genpdfi` (fork) | `0.2.7` | 2026-01-27 | Apache-2.0 | Rust | ✅ | ❌ | ❌ (font fallback chain only) | Active fork (~23k downloads) |
| `typst` (+ `typst-pdf`, embedded via `typst-as-lib` `0.15.4`) | `typst@0.14.2` / `typst-as-lib@0.15.4` | 2025-12-12 / 2026-01-29 | Apache-2.0 | Rust | ✅ | ⚠️ partial (typst-html exists; PDF path is the canonical one) | ✅ first-class (`text(dir: rtl)`, rustybuzz shaping) | Active (1.19M / 408k downloads) |
| `weasyprint-rs` | **does not exist** | — | — | — | ❌ | (n/a) | (n/a) | **No Rust binding** (Python only) |
| `wkhtmltopdf` (Rust crate `wkhtmltopdf@0.4.0`) | binary archived 2023 | last bin release 0.12.6 (2020-06) | LGPL-3.0 + bundled Qt 4 | C++ binary via subprocess | ❌ (binary dep) | ✅ (WebKit1, EOL) | ⚠️ depends on bundled WebKit | **Archived Jan 2023; org archived Jul 2024** |

**License notes** vs. LD-1 (MIT): MIT and Apache-2.0 are aligned with LD-1 and pass `cargo deny check licenses` allowlist (LD-37). LGPL-3.0 + bundled Qt 4 (wkhtmltopdf binary) does **not** pass — bundling LGPL into a Tauri desktop binary is a redistribution event with binding-time obligations the project has not signed up for, and the Qt 4 component has been EOL since 2015.

### Critical disqualification findings (verified 2026-05-19)

Two of the five candidates are immediately disqualified before criteria scoring:

#### `wkhtmltopdf` — Archived; supply-chain unacceptable

- The upstream `wkhtmltopdf/wkhtmltopdf` GitHub repository was **archived by the maintainer on 2023-01-02**; the entire GitHub organization was archived on **2024-07-10**.
- Last stable release was **0.12.6 (2020-06)**.
- Architecturally depends on **Qt 4.8.5 with patched QtWebKit** — QtWebKit was deprecated in 2015 and removed from Qt in 2016. No upstream receives security fixes for the embedded WebKit1 in-process renderer.
- Shipping wkhtmltopdf with Orgsidian in 2026 violates the supply-chain posture established by LD-37 (architecture.md:1055): `cargo audit` + `cargo deny` + dep graph CI check exist precisely to keep stale, unmaintained binary deps out of the tree. A bundled ~10-year-unpatched browser engine is the canonical case the policy was written to block.
- **Verdict:** disqualified on LD-37 grounds, independent of any criterion score.

#### `weasyprint-rs` — Does not exist as a Rust binding

- A query against crates.io returns `crate weasyprintrs does not exist`.
- WeasyPrint (`Kozea/WeasyPrint`) is a pure-**Python** library built on Pango (via cairo/GObject), Pydyf, tinycss2, and cssselect2 — none of which expose a stable C ABI suitable for direct Rust FFI.
- The only integration path is **spawning the `weasyprint` CLI as a subprocess**, which requires bundling (or requiring an installed) Python 3 runtime plus the native Pango/cairo/HarfBuzz stack alongside the Tauri desktop app. This violates:
  - **LD-2 (Tauri)** packaging posture — Tauri's value proposition is a single self-contained binary; piggybacking a Python runtime defeats the model.
  - **No-native-deps implicit posture** — every other LD selected pure-Rust where available (LD-26 `rusqlite` with bundled SQLite; LD-30 `tantivy` over external Lucene; LD-48 vendored `tree-sitter-org`).
  - **FR-14 latency intuition** — a per-render Python interpreter startup (≥200–500ms on cold cache) competes with the FR-10 <1s budget reused for export operations.
- WeasyPrint also carries an open CVE (CVE-2025-68616, `default_url_fetcher`) that would need independent tracking and would not benefit from Cargo's lockfile + `cargo audit` workflow.
- **Verdict:** disqualified on packaging-model and dep-hygiene grounds; the option as stated (`weasyprint-rs` as a Rust crate) does not exist.

#### `genpdf` (original) — Abandoned

- Last release `0.2.0` on **2021-06-17**, no commits since.
- The actively maintained fork is `genpdfi` (`0.2.7`, 2026-01-27). The remainder of this report treats the genpdf slot as **genpdfi**.

### Surviving candidates for criterion scoring

1. **`printpdf` 0.9.1** — pure-Rust PDF generator with experimental HTML/CSS rendering (`html` feature flag).
2. **`genpdfi` 0.2.7** — pure-Rust higher-level layout primitives (text blocks, tables, paragraphs); no HTML rendering.
3. **`typst` 0.14.2 embedded** — full typesetting system as a Rust library; consumed via `typst-as-lib` 0.15.4 for ergonomic in-process compilation.

---

## Step 3 — Integration Patterns (against `orgsidian-report` crate layout)

### Programmatic API surface (data → PDF flow)

`orgsidian-report` consumes `core` query results (`Vec<TodoCompletion>`, `Vec<ClockEntry>`, `Vec<BacklinkRef>`, `Vec<MilestoneStatus>`) and emits a `Vec<u8>` of PDF (or a stream into HTML). The three candidates differ structurally:

- **printpdf** — Two modes:
  - *Imperative ops vector* (`Vec<Op>`): direct PDF instruction stream — `SetFont`, `ShowText`, `AddLineBreak`, `SetFillColor`, etc. Suited for rigid, generator-driven layouts. Verbose for tabular reports.
  - *HTML mode* (`PdfDocument::from_html(html, &images, &fonts, &options, &mut warnings)`) — accepts an HTML string + CSS in `<style>` tags + named image/font registry; automatic page breaking; header/footer via `<head><header>…</header><footer>…</footer></head>` blocks. This is the printpdf path that aligns with OQ-6 (HTML/CSS template customization).
- **genpdfi** — Document-tree builder (`genpdfi::Document::new(font_family)` + `doc.push(genpdfi::elements::Paragraph::new("…"))`). Layout primitives: paragraphs, lists, tables, page breaks. No HTML pipeline. Suited for code-driven structured reports where Rust owns the layout.
- **typst (via `typst-as-lib`)** — Template-driven: a `.typ` source file (or string) defines layout in Typst's DSL; data flows in via `sys.inputs` (a JSON-like mapping populated programmatically). Compilation:
  ```rust
  let engine = TypstEngine::builder()
      .main_file(template_typ_source)
      .fonts(font_resolver)
      .build();
  let pdf_bytes: Vec<u8> = engine.compile_with_input(inputs_dict)?.into_pdf()?;
  ```
  Output is a `Vec<u8>` of PDF — no subprocess, no temp files (unless the consumer chooses to).

### Alignment with LEAF-crate rule (LD-37, architecture.md:1055)

`orgsidian-report` is declared a leaf crate (architecture.md:913). Consumer crates (`shell-app`, `cli`) cannot import it directly per `cargo-deny check graph`; only `orgsidian-core` reaches in. All three surviving candidates are pure-Rust and add no transitive crates that would re-emerge in CLI or shell-app dependency closures.

### Build-time & binary-size cost (qualitative, target: minimize incremental cost on `orgsidian-report` only)

| Candidate | Direct deps (approx.) | Heaviest transitive | Estimated PDF feature stripped-binary delta |
|---|---|---|---|
| `printpdf` (no `html`) | ~30 crates incl. `lopdf`, `image`, `time` | `lopdf`, optional `usvg` | small (~2–4 MB) |
| `printpdf` + `html` feature | adds `azul-css`/`xmlparser` for HTML/CSS parse | as above | small-to-medium (~4–6 MB) |
| `genpdfi` | ~15 crates incl. `rusttype`, `ttf-parser` | `rusttype` | very small (~1–2 MB) |
| `typst` + `typst-pdf` + `typst-as-lib` | ~150 crates incl. `comemo`, `rustybuzz`, `ttf-parser`, `icu_segmenter`, `wasmi` (typst plugin runtime) | `wasmi`, `comemo`, `icu_*`, `ecow` | **medium-to-large (~12–18 MB)** stripped, ~30–40 MB unstripped |

The typst delta is real but lands **only in `orgsidian-report`**, which by LD-14 is excluded from the `cli` binary's dependency closure. The Tauri desktop binary already weighs in at ~10–30 MB across the three target OSes; a further ~12–18 MB is acceptable for a wow-demo-tier feature.

### Tauri 2.x specifics

All three surviving candidates are pure-Rust and execute entirely in the Rust process; none invoke a subprocess, none load OS-level native libraries beyond what `tauri` itself already brings. CSP (LD-18) is unaffected (PDF generation never crosses the IPC boundary into the webview). No interaction with `tauri-specta` (LD-24). The PDF output reaches the user via `tauri_plugin_dialog` save-dialog + `tokio::fs::write` — same path for all three.

---

## Step 4 — Architectural Patterns (against locked LDs)

### Typography quality

Typography is the first criterion the brief lists, and the FR-14 acceptance criterion explicitly mentions "readable typography" (prd.md:275).

- **typst** is, by design, a **typesetting system** — it does line-breaking with the Knuth-Plass-derived algorithm, full Unicode segmentation via `icu_segmenter`, complex-script text shaping via `rustybuzz` (HarfBuzz Rust port), proper kerning, ligatures, optical alignment, hyphenation, and microtypography. Output is comparable to LaTeX/InDesign quality. Direct-from-design.
- **printpdf** uses `allsorts` or built-in shaping for custom fonts; supports Unicode codepoints via `TextItem::Text(String)` and font subsetting via `PdfSaveOptions { subset_fonts: true, … }`. No hyphenation engine. Line-breaking in the `html` path is the browser-engine-style box model in `azul-css` — adequate for simple paragraphs, less polished than typst for justified text and headings.
- **genpdfi** uses `rusttype` for glyph metrics; basic word-wrap; no hyphenation; no kerning beyond what `rusttype` provides. Typography quality is "competent for invoices", not "wow-demo professional".

**Verdict:** typst >> printpdf > genpdfi.

### Unicode / RTL / complex scripts

PRD §8 (prd.md:524) commits translation infrastructure for v1.0 with community-driven translations. The library universe for v1.0 must therefore handle Latin, Cyrillic, CJK, Arabic, Hebrew, Devanagari at minimum.

- **typst** — first-class:
  - Bidi via the `text(dir: rtl)` directive and per-document `set text(dir: rtl)`; mixed-direction runs via the Unicode bidi algorithm.
  - Complex shaping (Arabic ligatures, Indic conjuncts) via `rustybuzz`.
  - Built-in CJK support including vertical-writing mode and East Asian line-break rules.
  - Font fallback via `set text(font: ("Primary", "Fallback1", "Fallback2"))`.
- **printpdf** — Unicode codepoints render correctly if the font supplies the glyph (Russian example in the docs). Custom font loading with subsetting works. **Bidi/RTL is not advertised as a first-class feature** in the public docs; mixed-direction text would require pre-shaping by the consumer (or rely on whatever shaping `allsorts` performs, which is not Bidi-Algorithm-complete). Acceptable risk for European-script v0.5 Beta; questionable for v1.0 RTL coverage.
- **genpdfi** — font fallback chain via `FontFallbackChain` (genpdfi exposes `segment_text(&self, text: &str) -> Vec<(String, &FontData)>`), enabling per-script font selection. No bidi reordering. Latin-extended (Romanian) verified in docs; CJK and RTL would require consumer-side bidi pre-processing and are effectively unsupported.

**Verdict:** typst > printpdf > genpdfi for v0.5 Beta (Latin-only); typst >> printpdf > genpdfi for v1.0 (community translations including RTL/CJK).

### Native dependencies

All three surviving candidates are **pure-Rust, zero native dependencies** at runtime. Compilation requires only `cargo`; no system Pango, cairo, Python, Qt, or GObject. This is the headline advantage that disqualified weasyprint and wkhtmltopdf.

**Verdict:** three-way tie (all pure-Rust).

### HTML/CSS template render vs. primitive layout

The brief explicitly accepts **either** "capacità di rendere HTML-CSS template **oppure** capacità di costruire layout primitivo".

- **printpdf (`html` feature)** — Renders HTML/CSS directly. Single-format alignment with OQ-6 ("v1.0 commits to template files for HTML/CSS customization") if and only if printpdf is chosen for both PDF and HTML outputs — but Orgsidian's HTML output path is independent (probably static HTML emission from `core` data without going through printpdf at all), so this "alignment" is partly a phantom benefit. The HTML/CSS subset printpdf supports is a moving target; complex CSS (flexbox, grid, custom fonts via `@font-face`, modern selectors) may not render correctly.
- **typst** — Powerful, programmable layout DSL (`.typ` files). Custom report templates are written in Typst syntax. Variable substitution via `sys.inputs`. Not HTML/CSS. **OQ-6 customization template language for the PDF path would be `.typ`**, separate from any HTML template the HTML-output path uses. Two template formats for the customization story (HTML/CSS for HTML output, .typ for PDF output) — a documented tradeoff, not a violation.
- **genpdfi** — Layout primitives in Rust code (paragraphs, tables, page breaks). No template language at all — every report layout change is a Rust code change. OQ-6 customization story would require shipping a Rust mini-DSL or exposing the document tree to plugins, both of which are out-of-scope for v0.5 Beta.

**Verdict:** typst (programmable template DSL) > printpdf (HTML/CSS template) > genpdfi (Rust-code-only layout).

### Maintenance health & ecosystem (2026)

- **typst** — Open-source ETH Zurich-spun-off project, active dev team, 0.14.2 released 2025-12-12, on a regular quarterly-ish release cadence. `typst-as-lib` 0.15.4 (2026-01-29) is community-maintained but the upstream `typst` and `typst-pdf` crates are first-party. Production users in 2025/2026 include the Typst.app web playground (WASM-compiled), Tinymist LSP, and report-generation tools cited in the typst-as-lib README. 408k downloads for `typst-as-lib`, 1.19M for `typst`.
- **printpdf** — Maintained by `fschutt`, 0.9.1 released 2026-02-17, regular releases through 2025/2026. 1.25M cumulative downloads. The `html` feature is newer (introduced 2024) and undergoing active development.
- **genpdfi** — Fork of `genpdf` (abandoned 2021) by `theiskaa`, 0.2.7 released 2026-01-27. Smaller user base (23k downloads). Maintenance is single-maintainer; bus-factor concern for a v0.5 Beta dependency that lives in the wow-demo path.

**Verdict:** typst ≥ printpdf > genpdfi.

---

## Step 5 — Implementation Research: template authoring & data binding

### typst `.typ` template + `sys.inputs` (recommended path)

```typst
// orgsidian-report-default.typ
#let inputs = sys.inputs
#set document(title: inputs.project_title)
#set text(font: "Inter", size: 11pt, lang: inputs.lang)
#set page(paper: "a4", margin: 2cm, numbering: "1 / 1")

= #inputs.project_title
*Period:* #inputs.period_start — #inputs.period_end

== Completed TODOs
#table(
  columns: (auto, auto, 1fr),
  [*Date*], [*Headline*], [*Source*],
  ..for todo in inputs.todos { (todo.date, todo.headline, todo.source) }
)

== Clocked Time
*Total:* #inputs.total_hours h
#for entry in inputs.clock_entries [
  - #entry.headline — #entry.hours h
]

== Linked Notes (grouped by source file)
#for group in inputs.linked_notes [
  === #group.source_file
  #for ref in group.refs [
    - *#ref.headline* — _#ref.context_excerpt_
  ]
]

== Milestones
#for ms in inputs.milestones [
  - [#if ms.complete [x] else [ ]] #ms.headline (#ms.status)
]
```

```rust
// crates/orgsidian-report/src/typst_renderer.rs (sketch)
use typst_as_lib::TypstEngine;

pub fn render_project_report_pdf(data: &ReportData) -> Result<Vec<u8>, ReportError> {
    let template = include_str!("../templates/orgsidian-report-default.typ");
    let engine = TypstEngine::builder()
        .main_file(template)
        .fonts(embedded_font_resolver())
        .build();
    let inputs = serde_json::to_value(data)?;
    let pdf = engine.compile_with_input(inputs)?.into_pdf()?;
    Ok(pdf)
}
```

**Properties:**
- Template ships as `include_str!`-embedded asset in the `orgsidian-report` crate. No external file lookup at runtime; reproducible builds.
- Fonts (Inter, Noto Sans CJK, Noto Sans Arabic) ship embedded via `embedded_font_resolver()` returning an in-memory font set — same pattern as the typst playground.
- Data binding via `serde_json::to_value` on a strongly-typed `ReportData` struct (defined in `orgsidian-core`'s public API): compile-time-checked at the Rust side; runtime-checked at the .typ side via Typst's gradual typing.
- OQ-6 customization (v1.0): expose `templates/` directory under user data dir; ship default `orgsidian-report-default.typ`; allow users to drop in `orgsidian-report-user.typ` to override. The customization surface is the `.typ` language plus the documented `sys.inputs` schema (which we generate documentation for from the Rust `ReportData` struct).

### printpdf HTML/CSS template (rejected alternative, recorded for the LD)

```rust
let html = render_handlebars_template("project-report.hbs", &data)?;
let images = BTreeMap::new();
let fonts = orgsidian_fonts(); // BTreeMap<String, Vec<u8>>
let options = GeneratePdfOptions::default();
let mut warnings = Vec::new();
let doc = PdfDocument::from_html(&html, &images, &fonts, &options, &mut warnings)?;
let pdf_bytes = doc.save(&PdfSaveOptions { subset_fonts: true, ..Default::default() }, &mut Vec::new());
```

Workable, but introduces a templating-engine choice (`handlebars`, `tera`, `minijinja`) — none currently in the architecture — and the typography ceiling is the printpdf HTML renderer, which is markedly below typst.

### genpdfi document-tree (rejected alternative)

```rust
let mut doc = genpdfi::Document::new(font_family);
doc.set_title("Project Report");
doc.push(genpdfi::elements::Paragraph::new(format!("Period: {start} — {end}")));
doc.push(genpdfi::elements::TableLayout::new(vec![1, 2, 3])
    .row(vec!["Date", "Headline", "Source"])
    /* ... */ );
doc.render_to_file("report.pdf")?;
```

Every layout iteration requires a recompile. No path to OQ-6 (v1.0 customization). Not viable as the FR-14 wow-demo renderer.

---

## Step 6 — Research Synthesis

### Weighted decision matrix

Criteria weighted by emphasis in PRD §4.3 FR-14 and the user's brief (typography first, Unicode/RTL second, then native deps, binary size, template capability, maintenance):

| Criterion | Weight | typst | printpdf | genpdfi |
|---|---|---|---|---|
| Typography quality | 3 | **5** | 3 | 2 |
| Unicode / Bidi / CJK | 3 | **5** | 3 | 2 |
| Native deps (zero is the bar) | 2 | 5 | 5 | 5 |
| Binary size on `orgsidian-report` | 2 | 2 | **4** | **5** |
| HTML/CSS or programmable template | 2 | **5** (Typst DSL) | 4 (HTML/CSS) | 2 (Rust code only) |
| Maintenance health 2026 | 2 | **5** | 4 | 3 |
| **Weighted total (max 70)** | | **62** | 52 | 42 |

typst leads decisively even with the binary-size penalty — and the penalty is contained within the LEAF crate that LD-14 explicitly extracted to absorb such costs.

### Decision

**Adopt `typst` as the PDF rendering engine for `orgsidian-report`**, embedded as a Rust library via the `typst-as-lib` wrapper (with `typst` and `typst-pdf` as transitive deps). Pin versions per the project version policy memory ([[feedback_version_policy]]): `typst@0.14`, `typst-pdf@0.14`, `typst-as-lib@0.15`. HTML output (the second format FR-14 commits to) is generated by a separate path (static templated HTML emission from `core` data via a small handlebars/minijinja templater chosen during the FR-14 sprint — out of scope for this LD).

Rationale (mapped to PRD constraints):

1. **PRD §4.3 FR-14 typography clause** — "readable typography" is best satisfied by a typesetting system, not a PDF generator with bolted-on HTML rendering.
2. **PRD §8 v1.0 translations** — typst's first-class bidi/CJK shaping pre-pays the Arabic/Hebrew/Chinese coverage cost. The alternative (community RTL translator finds the PDF unreadable in v1.0) is a wow-demo regression we can avoid for ~12–18 MB of binary.
3. **LD-1 (MIT)** — typst is Apache-2.0, allowlist-compatible; `typst-as-lib` is Apache-2.0.
4. **LD-2 (Tauri)** — pure-Rust, in-process, single-binary; no Python/Qt sidecar.
5. **LD-14 (`orgsidian-report` extraction)** — the cost the LD pre-absorbed is exactly the typst-class dep weight; CLI remains free of the cost.
6. **LD-37 (dep hygiene)** — typst's transitive closure (~150 crates) is large but `cargo audit`/`cargo deny` clean; no unmaintained or unauthorized-license deps surfaced in spot-check; full audit happens on first CI run.
7. **OQ-6 (v1.0 template customization)** — customization surface becomes "user-supplied `.typ` template with a documented `sys.inputs` schema". This is a real change from the PRD wording (which said "HTML/CSS"), and is recorded in the LD-53 amendment as an OQ-6 reconciliation note.

### Risks & mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| typst binary-size delta (~12–18 MB) is visible in `orgsidian-report` build, but LEAF-crate isolation keeps it out of `cli` | High (it will happen) | LD-14 already absorbed the delta intent; record actual measured delta in v0.5 Beta sprint and confirm it stays inside the desktop-app size envelope |
| OQ-6 (PRD) said "HTML/CSS customization"; we now ship "`.typ` customization" for the PDF path | Medium | LD-53 records the reconciliation; PRD reconcile pass (LD-46) covers it in the same wave as the MIT/tree-sitter-org reconciliation tracked at architecture.md:1268 |
| `typst-as-lib` is community-maintained (single-author); upstream `typst` is first-party | Medium | If `typst-as-lib` lags, the orgsidian-report crate can call `typst` + `typst-pdf` directly (~80 LoC of glue); contingency documented but not pre-implemented |
| `.typ` learning curve for theme authors (vs HTML/CSS) | Medium (v1.0 issue, not v0.5) | v0.5 Beta ships one canonical template only; OQ-6 v1.0 work includes a "writing report templates" doc page; .typ syntax is simpler than LaTeX |
| typst `0.x` version cadence may include breaking changes | Medium | [[feedback_version_policy]] mandates "latest stable" — accept and apply changes during the quarterly Tauri-sync window (LD-47), same cadence |
| Embedded font assets add ~5–10 MB (Inter + Noto subsets) | Medium | Use Noto **subset** files (Latin + Latin-Ext + Cyrillic for v0.5 Beta; expand to CJK/Arabic in v1.0); font subsetting at compile time keeps the delta bounded |

### Decision confidence

**HIGH** — three of the five candidates are immediately disqualified on hard grounds (wkhtmltopdf archived; weasyprint-rs nonexistent; genpdf abandoned); of the two-and-a-half remaining (printpdf, genpdfi, typst), typst wins decisively on the two heaviest-weighted criteria (typography and Unicode/RTL) and is acceptable on the rest. The only tradeoff the project absorbs is binary size, which LD-14 pre-paid for.

---

## LD-53 Amendment (copy-paste-ready for `architecture.md`)

> **LD-53. `orgsidian-report` PDF rendering: `typst` embedded via `typst-as-lib`.**
>
> **Decision.** The `orgsidian-report` crate (LD-14) renders FR-14 Project Report PDFs using the Typst typesetting system embedded as a Rust library. Direct deps pinned per the project version policy: `typst@0.14`, `typst-pdf@0.14`, `typst-as-lib@0.15` (Apache-2.0; allowlist-compatible per LD-1 / LD-37). All rendering is in-process; no subprocess, no native deps, no Python or Qt runtime. Closes the Important Gap at architecture.md:1267 and the "Areas for Future Enhancement" note at architecture.md:1332.
>
> **Why typst over the other four spike candidates** (verified 2026-05-19, research at `_bmad-output/planning-artifacts/research/technical-pdf-rendering-crate-selection-research-2026-05-19.md`):
>
> - **`wkhtmltopdf` (subprocess) — disqualified.** Upstream repository archived 2023-01-02; org archived 2024-07-10; last binary release 2020-06; depends on Qt 4.8.5 + patched QtWebKit (EOL since 2015). Shipping an unmaintained ~10-year-unpatched in-process browser engine violates the supply-chain hygiene posture set by LD-37.
> - **`weasyprint-rs` — does not exist.** No Rust crate of that name on crates.io; no Rust FFI binding to WeasyPrint exists. Only integration path is a Python subprocess, which requires bundling a Python 3 runtime + native Pango/cairo/HarfBuzz alongside the Tauri binary — incompatible with the LD-2 single-binary packaging posture and the no-native-deps pattern established by LD-26 / LD-30 / LD-48.
> - **`genpdf` (original) — abandoned.** Last release 0.2.0 in 2021-06; no commits since. Active fork is `genpdfi` 0.2.7 (2026-01).
> - **`genpdfi`** — pure-Rust, maintained, small footprint, but layout is Rust-code-only (no template surface for OQ-6 customization), no bidi/RTL shaping (font fallback chain only), typography baseline materially below typst. Not adequate for FR-14 wow-demo bar at PRD §4.3 acceptance.
> - **`printpdf` 0.9.1** — strong second choice; pure-Rust, maintained, includes an experimental `html` feature (`PdfDocument::from_html(...)`) that would align cleanly with OQ-6's HTML/CSS template intent. Loses on (a) typography polish (general-purpose PDF generator, not a typesetting engine), (b) bidi/RTL not first-class — adequate for Latin-script v0.5 Beta but a v1.0 PRD §8 community-translation liability, (c) the `html` feature's CSS subset is a moving target less suited to a "must look professional" wow demo. Retained as the documented fallback if a v0.5 Beta typst risk materializes (see "Downgrade path" below).
>
> **Why typst is the right fit, mapped to locked LDs and PRD:**
>
> - **FR-14 acceptance — "readable typography"** (prd.md:275). Typst is a typesetting system (Knuth-Plass line-breaking, `rustybuzz` shaping, kerning, ligatures, hyphenation). Output is comparable to LaTeX/InDesign quality; the FR-14 wow demo lands as a wow demo.
> - **PRD §8 community translations (v1.0).** First-class bidi via `text(dir: rtl)` and the Unicode Bidi Algorithm; complex-script shaping (Arabic, Indic, CJK) via `rustybuzz` + `icu_segmenter`. Pre-pays the Arabic/Hebrew/Chinese translator-coverage cost without a v1.0 renderer-swap project.
> - **LD-1 (MIT) / LD-37 (`cargo deny` license allowlist).** `typst`, `typst-pdf`, `typst-as-lib` are all Apache-2.0 (allowlist-aligned). Transitive closure (~150 crates) is large but no unmaintained or unauthorized-license deps surfaced in spot-check; first CI run executes the full `cargo audit` + `cargo deny` sweep.
> - **LD-2 (Tauri 2.x).** Pure-Rust, in-process, no Python/Qt sidecar; single-binary distribution preserved across macOS / Windows / Linux.
> - **LD-14 (`orgsidian-report` crate extraction).** The crate was extracted precisely to absorb heavy PDF deps; typst's binary-size delta (~12–18 MB stripped on `orgsidian-report`) lands inside the LEAF crate and stays out of `cli`'s dependency closure per `cargo deny check graph`.
> - **[[feedback_version_policy]] (latest-stable pinning; quarterly Tauri-sync window per LD-47).** Typst's 0.x cadence aligns with the LD-47 quarterly bump rhythm; potential breaking changes batched into the same window.
>
> **Implementation outline** (target: `crates/orgsidian-report/`):
>
> ```
> crates/orgsidian-report/
> ├── Cargo.toml                          # deps: typst = "0.14", typst-pdf = "0.14", typst-as-lib = "0.15", serde, serde_json
> ├── src/
> │   ├── lib.rs                          # pub fn render_project_report_pdf(data: &ReportData) -> Result<Vec<u8>, ReportError>
> │   ├── pdf_renderer.rs                 # typst engine setup + compile_with_input
> │   ├── html_renderer.rs                # static HTML emission (separate path; templater choice deferred to FR-14 sprint, out of LD-53 scope)
> │   ├── fonts.rs                        # embedded_font_resolver(): Inter + Noto Sans (Latin/Cyrillic) for v0.5 Beta; CJK/Arabic in v1.0
> │   └── data.rs                         # ReportData struct (mirrors core query API), derive Serialize
> └── templates/
>     └── orgsidian-report-default.typ    # bundled via include_str!; ships as v0.5 Beta default
> ```
>
> Data flow: `core` returns a `ReportData` struct → `serde_json::to_value` → `TypstEngine::compile_with_input(inputs)` → `Vec<u8>` PDF → `tauri_plugin_dialog` save dialog → `tokio::fs::write`. HTML output uses a parallel `html_renderer.rs` path (not typst-html); choice of HTML templater (`handlebars` vs `minijinja` vs `tera`) deferred to the FR-14 sprint and recorded as an in-sprint micro-decision, out of LD-53 scope.
>
> **Embedded fonts (v0.5 Beta):** Inter (Variable) for sans-serif body; JetBrains Mono for code blocks; Noto Sans subset (Latin + Latin-Ext + Cyrillic) as fallback. All OFL-licensed. Total embedded font payload target: ≤8 MB. v1.0 adds Noto Sans CJK SC + Noto Sans Arabic subsets when PRD §8 translation rollout begins (separate LD).
>
> **OQ-6 reconciliation (v1.0 customization template language).** PRD §4.3 OQ-6 (prd.md:545) stated: "v1.0 commits to template files for HTML/CSS customization. The exact template variable surface is unspecified. Resolution: drafted in v0.5 spike based on Beta tester feedback." LD-53 changes the PDF-path customization surface from HTML/CSS to **Typst `.typ` templates with a documented `sys.inputs` schema**; the HTML-path customization surface remains HTML/CSS. OQ-6 wording therefore needs reconciliation in the same PRD pass as LD-46 (architecture.md:1268). The drafting deliverable in v0.5 Beta is now: (a) `orgsidian-report-default.typ` shipped + (b) `docs/customization/report-templates.md` documenting the `sys.inputs` schema generated from the `ReportData` struct.
>
> **Downgrade path (recorded contingency, not pre-implemented).** If the v0.5 Beta sprint surfaces a typst blocker (build-time regression beyond the LEAF-crate envelope, `cargo deny` license rejection on a transitive dep, or a typst-side regression in `rustybuzz` shaping that ships in 0.14.x), the contingency is `printpdf` 0.9.x with the `html` feature: same `orgsidian-report` crate layout, swap `typst_renderer.rs` for a `printpdf_renderer.rs` consuming HTML templates rendered via a small templater. Expected swap cost: ~3 dev-days. Confidence this contingency is not invoked: HIGH (typst is production-validated as an embedded library in 2025/2026 per Tinymist, Typst.app web playground, and `typst-as-lib` downstream telemetry).
>
> **Closes:** architecture.md:1267 (Important Gap #1, PDF crate selection); architecture.md:1332 (Areas for Future Enhancement).
> **Touches PRD:** §4.3 OQ-6 customization-template language — reconciliation tracked in the same wave as LD-46.
> **Decision date:** 2026-05-19.

---

## Sources (verified 2026-05-19)

- **crates.io API (versions, dates, downloads):**
  - `printpdf` 0.9.1 (2026-02-17, 1,245,833 downloads)
  - `genpdf` 0.2.0 (2021-06-17, abandoned)
  - `genpdfi` 0.2.7 (2026-01-27, 23,446 downloads)
  - `typst` 0.14.2 (2025-12-12, 1,192,812 downloads)
  - `typst-as-lib` 0.15.4 (2026-01-29, 408,657 downloads)
  - `wkhtmltopdf` (Rust crate) 0.4.0 (2021-05-04, abandoned)
  - `weasyprint-rs` — crate does not exist (`{"errors":[{"detail":"crate weasyprint-rs does not exist"}]}`)
- **Upstream documentation via `ctx7`:**
  - `/fschutt/printpdf` — operations API, HTML feature, font handling.
  - `/websites/rs_genpdfi_0_2_7` — font fallback chain, subsetting, layout primitives.
  - `/typst/typst` — embeddable Rust crate; `typst compile` semantics; PDF/A and PDF/UA support.
  - `/kozea/weasyprint` — Pango/cairo/Python dependency stack; Fontconfig requirement.
  - `/websites/rubydoc_info_gems_wicked_pdf` — wkhtmltopdf subprocess invocation pattern.
- **Upstream archive/EOL status (verified via research agent):**
  - wkhtmltopdf repository archived 2023-01-02; org archived 2024-07-10; Qt 4 + QtWebKit EOL since 2015–2016.
  - WeasyPrint open CVE CVE-2025-68616 (`default_url_fetcher`).
  - typst embeddable as library: Typst.app web playground (WASM), Tinymist LSP, `typst-as-lib`, `typst-as-library`.

---

**End of research report.**
