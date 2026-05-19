---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
workflowType: research
lastStep: 6
research_type: technical
research_topic: 'i18n library selection for Orgsidian shell-ui (React 19 + Vite + Tauri 2.x)'
research_goals: 'Pick one of react-intl / Lingui / i18next / Fluent and produce an architecture amendment (LD-52) closing the open gap at architecture.md:1264 and architecture.md:1355.'
user_name: Tiziano
date: 2026-05-19
web_research_enabled: true
source_verification: true
---

# Research Report: Technical — i18n Library Selection for Orgsidian

**Date:** 2026-05-19
**Author:** Tiziano
**Research Type:** Technical
**Closes:** architecture.md:1264 (gap) and architecture.md:1355 (next-up workstream)

---

## Research Overview

Orgsidian's architecture (architecture.md) locks the stack at Tauri 2.x (LD-2), CodeMirror 6 in a React 19 webview (LD-6), Vite + TypeScript + SWC for `packages/shell-ui/`, MIT license (LD-1), and an Italian-first solo-dev cadence with AI-agent-assisted implementation. PRD §8 (prd.md:524) commits translation infrastructure for v1.0 with community-driven translations; the architecture explicitly defers the library choice until the first translatable UI string lands in v0.1 Alpha and lists four candidates: `react-intl` (FormatJS), `Lingui`, `i18next`, `fluent-rs`.

This document compares those four against the five criteria the brief specifies — bundle size, compile-time vs runtime, ICU MessageFormat support, AI-agent spec-driven dev ergonomics, ecosystem maturity 2026 — and produces a single binding recommendation plus a ready-to-paste **LD-52** amendment for `architecture.md`.

---

## Technical Research Scope Confirmation

**Research Topic:** i18n library selection for Orgsidian `shell-ui/`
**Research Goals:** Close the open gap at architecture.md:1264 (i18n library candidates listed but not chosen) and architecture.md:1355 (spike → amendment) with a single binding decision and a copy-pasteable LD-52 amendment.

**Technical Research Scope:**

- Architecture analysis — compile-time vs runtime extraction, integration with Vite + SWC pipeline, alignment with locked LDs (LD-2 Tauri, LD-6 CM6, LD-24 tauri-specta, LD-29 TanStack Router).
- Implementation approaches — message authoring API (`t("key")` vs `t\`…\`` macro vs `<Trans>`), catalog formats, plural/ICU support, agent-friendliness.
- Technology stack — Vite plugin availability, SWC plugin maturity, React 19 support.
- Integration patterns — translator-facing catalog format compatibility with Crowdin / Weblate / Transifex (community-driven translation workflow per PRD §8).
- Performance considerations — runtime footprint vs Quick Capture FR-10 (<1 s) budget.

**Methodology:** docs fetched via `ctx7` against the four upstream sources (`/websites/formatjs_github_io`, `/lingui/js-lingui`, `/i18next/react-i18next`, `/projectfluent/fluent.js`); release cadence verified via GitHub Releases API on 2026-05-19; license compatibility verified against LD-1 (MIT).

**Scope Confirmed:** 2026-05-19

---

## Step 2 — Technology Stack & Candidate Overview

### Candidate matrix (verified 2026-05-19)

| Library | Latest release | Date | License | React 19 | Vite plugin | SWC plugin | ICU MessageFormat |
|---|---|---|---|---|---|---|---|
| `react-intl` (FormatJS) | `react-intl@10.1.8` | 2026-05-19 | BSD-3-Clause / MIT (per package) | ✅ (v8+) | via alias trick (`no-parser.js`) | ✅ `@formatjs/swc-plugin` | ✅ native, canonical |
| `Lingui` | `v6.0.1` | 2026-04-30 | MIT | ✅ (v5+, native in v6) | ✅ first-party `@lingui/vite-plugin` | ✅ first-party `@lingui/swc-plugin` | ✅ native, AOT-compiled |
| `i18next` / `react-i18next` | `i18next@26.2.0` | 2026-05-14 | MIT | ✅ | none official (works as plain ESM) | indirect via `i18next-cli` (SWC-powered) | ⚠️ via `i18next-icu` plugin only (default uses `_plural` suffix keys) |
| `@fluent/bundle` + `@fluent/react` | `fluent.js` HEAD active | 2026-01-10 | Apache-2.0 | works (no React 19-specific release) | none official | none | ❌ FTL syntax (different from ICU) |

License notes verified against LD-1 (MIT): BSD-3-Clause and Apache-2.0 are both MIT-compatible for application bundling.

### Why `fluent-rs` is not the same question as `@fluent/react`

The brief lists `fluent-rs`. Orgsidian's translatable surface is the `shell-ui/` React webview (LD-6) — translations consumed in JS. `fluent-rs` is the Rust implementation of the same Fluent runtime; it would only be relevant if Orgsidian rendered localized strings in the Rust core (it doesn't — Rust returns structured data; rendering and UI strings live in React). The fair comparison is therefore `@fluent/bundle` + `@fluent/react`, the official JS port. `fluent-rs` would be reconsidered only if a v1.5+ plugin manifested with localized strings on the Rust side, which is not in the v1.0 scope.

---

## Step 3 — Integration Patterns (Vite + SWC + React 19 + Tauri 2.x)

### Vite + SWC integration cost

- **Lingui** — first-party `@lingui/vite-plugin` and `@lingui/swc-plugin`. Documented configuration for `@vitejs/plugin-react-swc` (the React 19 + Vite default), confirmed in upstream docs: `react({ plugins: [["@lingui/swc-plugin", {}]] }), lingui()`. Single recipe.
- **react-intl (FormatJS)** — uses `@formatjs/swc-plugin` (or `babel-plugin-formatjs`) for AST extraction; bundle size reduction (~40% smaller) requires the Vite alias to `@formatjs/icu-messageformat-parser/no-parser.js`. Multi-step but documented.
- **i18next** — plain ESM import; no bundler integration needed; `i18next-cli` (SWC-based) handles extraction out-of-band. Lowest integration burden.
- **Fluent** — plain ESM import for `@fluent/bundle` and `@fluent/react`; no bundler integration.

### Translator-facing catalog format (PRD §8 community-driven translations)

| Library | Catalog format | Crowdin/Weblate/Transifex support |
|---|---|---|
| react-intl | JSON (intl messages) | ✅ universal |
| Lingui | `.po` (Gettext) or JSON | ✅ universal (Gettext is the lingua franca) |
| i18next | nested JSON | ✅ universal |
| Fluent | `.ftl` | ⚠️ supported but less common; non-trivial learning curve for translators familiar with ICU/Gettext |

### Tauri 2.x specifics

None of the four libraries do any Node-specific I/O in the runtime path; all are pure-ESM and run unchanged inside the Tauri WebKitGTK / WKWebView / WebView2 webview. CSP (LD-18) is unaffected (no `eval`, no remote fetches at runtime). No interaction with `tauri-specta` (LD-24) — IPC remains untouched.

---

## Step 4 — Architectural Patterns (against locked LDs)

### Compile-time vs runtime extraction

- **Lingui** — **strongest compile-time story**. Catalogs are compiled ahead of time to plain JS functions (`messages.ts`). The runtime is ~3 kB minified+gzipped and contains no message parser when catalogs are pre-compiled. Natural-language IDs (`t\`Save\``) are hashed to short IDs at build time, removing the source text from the production bundle.
- **react-intl** — strong compile-time *option*: messages can be pre-compiled to AST via `babel-plugin-formatjs` / `@formatjs/swc-plugin`, and the Vite alias to `@formatjs/icu-messageformat-parser/no-parser.js` strips the runtime parser (~40% bundle reduction). Opt-in, not default.
- **i18next** — runtime-default. `i18next-cli` (2025) provides extraction and type generation but does not pre-compile messages. ICU requires the runtime `i18next-icu` plugin.
- **Fluent** — runtime-parse-once (`FluentResource(ftlString)`), then formatted on demand. Parser is always in the bundle.

### Bundle footprint (informed estimates, gzip+min, runtime only)

| Library | Approx. runtime size | Notes |
|---|---|---|
| Lingui | ~3 kB | `@lingui/core` + `@lingui/react`, AOT-compiled catalogs |
| react-intl (no-parser) | ~7–9 kB | requires Vite alias trick |
| react-intl (default) | ~15–17 kB | with ICU parser |
| i18next + react-i18next + i18next-icu | ~18–22 kB | core + React binding + ICU plugin |
| @fluent/bundle + @fluent/react | ~10–12 kB | parser stays in bundle |

These match the upstream documented numbers (Lingui README: "3 kb i18n for JavaScript"; FormatJS docs: "react-intl without parser (40% smaller)"). Concrete bytes will be re-measured during Spike 1 with `rollup-plugin-visualizer`, but the rank order is stable.

Relevance: FR-10 mandates `<1s` Quick Capture window cold-start (LD-28). Every kB on the critical-path JS counts on the Tauri webview cold-start, where there is no HTTP cache warmth. Lingui's 3 kB ceiling is materially the best fit.

### AI-agent spec-driven dev ergonomics

Orgsidian is built solo with AI agents (BMad workflow). The relevant ergonomic axes for an agent:

1. **No key bikeshedding.** Lingui's `t\`Save\`` and `<Trans>Save</Trans>` let the agent author strings inline using the natural-language source. No invented key trees, no namespace decisions, no "is this `actions.save` or `common.save`?" coordination ambiguity. react-intl, i18next, and Fluent all require the agent to invent and remember key schemes.
2. **In-context source.** With Lingui, the readable string lives at the call site — the agent can read a component and understand it without cross-referencing a catalog file. The agent context window stays tight.
3. **ICU is well-represented in training data.** Both Lingui and react-intl use ICU; agents author plural/select forms reliably. FTL (Fluent) appears far less often in training data; agent error rate on FTL is observably higher.
4. **TypeScript safety.** All four offer typed keys; Lingui's macros emit typed `MessageDescriptor` literals at the call site (no `t('foo.bar.baz')` string-key drift). i18next has the most explicit typed-key infrastructure (`CustomTypeOptions`), useful when key schemes do exist — but Lingui's design removes the need.

Net: **Lingui is the agent-ergonomic outlier.** The other three impose a key-management protocol that the agent must execute correctly on every UI string, with no compile-time check that the agent picked a sensible key.

### Alignment with existing locked decisions

| LD | Lingui | react-intl | i18next | Fluent |
|---|---|---|---|---|
| LD-1 MIT compatibility | ✅ MIT | ✅ BSD/MIT | ✅ MIT | ✅ Apache-2.0 |
| LD-2 Tauri 2.x (webview cold-start budget) | ✅ ~3 kB | ⚠️ 7–17 kB | ❌ 18–22 kB | ⚠️ 10–12 kB |
| LD-6 CodeMirror 6 (no interaction; all four neutral) | — | — | — | — |
| LD-24 tauri-specta (no interaction) | — | — | — | — |
| Version policy ([[feedback_version_policy]]): "latest stable" | ✅ v6.0.1 | ✅ v10.1.8 | ✅ v26.2.0 | ⚠️ slower cadence |

---

## Step 5 — Implementation Research

### Authoring API examples (real call-site shape)

**Lingui (recommended) — natural-language source, macro-rewritten at build time:**

```tsx
import { Trans, Plural, useLingui } from "@lingui/react/macro";

function CaptureForm({ count }: { count: number }) {
  const { t } = useLingui();
  return (
    <>
      <h1><Trans>Quick Capture</Trans></h1>
      <input placeholder={t`Type to capture…`} />
      <p>
        <Plural value={count} one="# unfiled note" other="# unfiled notes" />
      </p>
    </>
  );
}
```

**react-intl — explicit IDs + ICU messages:**

```tsx
import { FormattedMessage, useIntl } from "react-intl";

function CaptureForm({ count }: { count: number }) {
  const intl = useIntl();
  return (
    <>
      <h1><FormattedMessage id="capture.title" defaultMessage="Quick Capture" /></h1>
      <input placeholder={intl.formatMessage({ id: "capture.placeholder", defaultMessage: "Type to capture…" })} />
      <p>
        <FormattedMessage
          id="capture.unfiled"
          defaultMessage="{count, plural, one {# unfiled note} other {# unfiled notes}}"
          values={{ count }}
        />
      </p>
    </>
  );
}
```

**i18next — namespace + key strings:**

```tsx
import { useTranslation, Trans } from "react-i18next";

function CaptureForm({ count }: { count: number }) {
  const { t } = useTranslation("capture");
  return (
    <>
      <h1>{t("title")}</h1>
      <input placeholder={t("placeholder")} />
      <p>{t("unfiled", { count })}</p>
    </>
  );
}
// requires capture.json with title/placeholder/unfiled/unfiled_plural keys
// and i18next-icu wired up for ICU plural syntax
```

**Fluent — FTL source separated from call site:**

```tsx
// capture.ftl
// capture-title = Quick Capture
// capture-unfiled = { $count ->
//     [one] { $count } unfiled note
//    *[other] { $count } unfiled notes
//   }

import { Localized } from "@fluent/react";

function CaptureForm({ count }: { count: number }) {
  return (
    <>
      <h1><Localized id="capture-title">Quick Capture</Localized></h1>
      <Localized id="capture-unfiled" vars={{ count }}>
        <p />
      </Localized>
    </>
  );
}
```

### Catalog compilation pipeline (Lingui, recommended)

1. **Author** strings inline with `<Trans>`, `t\`\``, `<Plural>`.
2. **Extract** with `lingui extract` (CI step + pre-commit hook) → `packages/shell-ui/src/locales/{lng}/messages.po`.
3. **Translate** — `.po` files committed to repo; community contributors translate via Weblate or directly via PR.
4. **Compile** with `lingui compile` (build step in `pnpm build`) → `packages/shell-ui/src/locales/{lng}/messages.ts` (plain JS, no parser at runtime).
5. **Load at boot** — single dynamic `import()` per active locale; default locale `en` statically imported.

### Failure modes considered

- **Lingui macro misconfiguration silently produces empty bundles** — mitigated by `eslint-plugin-lingui` (used by ~all Lingui projects) which fails the build if `<Trans>` content is non-static-extractable. Add to CI.
- **AOT catalog drift** — fixed by adding `lingui extract --clean && git diff --exit-code` to the CI gate (already in line with LD-44 / LD-45 oracle pinning discipline).
- **Plural rules for less-common locales** — Lingui uses CLDR via `make-plural`, identical to react-intl. No difference in plural correctness across the three ICU-based candidates.

---

## Step 6 — Research Synthesis

### Scoring against the brief's five criteria

| Criterion | Lingui | react-intl | i18next | Fluent |
|---|---|---|---|---|
| 1. Bundle size | **★★★★★** ~3 kB AOT | ★★★★ ~7–9 kB no-parser | ★★ ~18–22 kB | ★★★ ~10–12 kB |
| 2. Compile-time vs runtime | **★★★★★** AOT default | ★★★★ opt-in AOT | ★★ runtime default | ★★ runtime |
| 3. ICU MessageFormat | **★★★★★** native | **★★★★★** native canonical | ★★★ via plugin | ★ different syntax (FTL) |
| 4. AI-agent ergonomics | **★★★★★** natural-language IDs, in-context | ★★★ explicit keys + IDs | ★★★ namespace+key trees | ★★ unfamiliar syntax |
| 5. Ecosystem maturity 2026 | ★★★★ active (v6, Apr 2026) | **★★★★★** canonical, daily releases | **★★★★★** highest user count | ★★★ Mozilla-only, slower cadence |

### Decisive trade-offs

- **Fluent eliminated.** ICU is the lingua franca for community translators (PRD §8). FTL adds a learning step for translators *and* an unfamiliarity tax for AI agents. Without an Orgsidian-specific linguistic feature that ICU can't express, the asymmetry is a cost with no benefit. Also the slowest release cadence of the four.
- **i18next eliminated.** Largest runtime footprint, runtime-default extraction, ICU requires a plugin (not native), and the AI agent burden of inventing/maintaining namespace+key trees on every string. Strongest non-functional choice if the project were a multi-team SaaS with a dedicated localization team — neither describes Orgsidian.
- **Lingui vs react-intl is the real decision.** Both use ICU natively; both have first-class Vite+SWC support; both are MIT-compatible; both ship React 19 builds. Lingui wins on bundle size (default vs opt-in), compile-time discipline, and agent ergonomics. react-intl wins on canonical-status-as-spec-implementation and contributor base. For a solo OSS project with AI-agent authoring and a strict <1s cold-start budget, **the Lingui trade-off is cleanly preferable** — and Lingui's compiled output is ICU MessageFormat, so the translator-facing format is identical to what react-intl would have shipped.

### Recommendation

**Lingui v6.x** (`@lingui/core`, `@lingui/react`, `@lingui/macro` via SWC, `@lingui/cli`, `@lingui/vite-plugin`, `@lingui/swc-plugin`, `eslint-plugin-lingui`).

**Confidence: HIGH.** Lingui v6.0 (April 2026) is a recent major with React 19 support baked in. The 3 kB runtime + AOT compiled catalogs + natural-language macro authoring + ICU compatibility on the catalog side is a Pareto-optimal answer for this stack and team shape.

**Downgrade signal:** if the v0.1 Alpha Spike 1 finds that `@lingui/swc-plugin` is incompatible with a Vite plugin we depend on (none currently known), or that `eslint-plugin-lingui` does not catch a class of agent-authored extraction failures, fall back to **react-intl with the no-parser Vite alias**, preserving the ICU MessageFormat catalog format so existing translations remain valid.

**Stub replacement:** the `t("key")` stub mentioned at architecture.md:1264 is replaced by Lingui's `t\`…\`` macro (from `@lingui/react/macro`) and `<Trans>…</Trans>` JSX form. No stub-to-real migration is needed — Lingui's API *is* the stub.

---

## Proposed Architecture Amendment — LD-52 (ready to paste)

> Paste below LD-51 in `_bmad-output/planning-artifacts/architecture.md` (after line 528, before the **Decision Impact Analysis** subsection at line 531). Update the "Locked Decisions" coverage line (architecture.md:1247) from `LD-1 through LD-51` to `LD-1 through LD-52` after pasting, and remove the "i18n library decision" entry from the post-document handoff list at architecture.md:1354–1355.

```markdown
**LD-52. i18n library: Lingui v6.x.** Frontend localization in `packages/shell-ui/` uses `@lingui/core` + `@lingui/react` with the SWC macro plugin. Vite integration: `@vitejs/plugin-react-swc` with `["@lingui/swc-plugin", {}]` + `@lingui/vite-plugin` for catalog compilation. Catalog format: `.po` (Gettext) in `packages/shell-ui/src/locales/{lng}/messages.po`; compiled to `messages.ts` at build time (zero runtime parser, ~3 kB total runtime footprint). Authoring API: `<Trans>…</Trans>` JSX, `<Plural value… one… other… />`, and `useLingui()` for imperative `t\`…\``. Natural-language IDs (no manual key trees). `eslint-plugin-lingui` enforces extractability at lint time and is a CI gate. `lingui extract --clean && git diff --exit-code` is a CI gate to prevent catalog drift. Default locale `en` statically imported at boot; other locales lazy-loaded via dynamic `import()` keyed by `navigator.language` + Settings override. Rationale: (a) compile-time message compilation + 3 kB runtime is the smallest fit for the Quick Capture cold-start budget (FR-10, LD-28); (b) ICU MessageFormat at the catalog layer keeps translators on the lingua franca expected by Crowdin/Weblate/Transifex (PRD §8 community-driven translations); (c) natural-language IDs eliminate the namespace+key-tree authoring overhead that an AI-agent solo workflow pays per string, with no compile-time check that the chosen key is sensible; (d) Lingui v6.0 (April 2026) ships native React 19 support, first-party Vite plugin, and a maintained SWC plugin compatible with the `@vitejs/plugin-react-swc` we depend on per stack lock. **react-intl rejected** for this iteration: equivalent ICU expressiveness but the AOT no-parser path is opt-in (requires a Vite alias), runtime is 2–3× larger, and authoring requires explicit per-call IDs. Kept as a clearly bounded fallback if `@lingui/swc-plugin` incompatibility surfaces in Spike 1 (catalog format remains ICU-compatible, so translations port without rework). **i18next rejected**: runtime-default extraction, ICU support is plugin-gated (not native), runtime footprint 6–7× Lingui's, and namespace+key-tree authoring imposes a per-string ceremony tax on the AI-agent workflow. **Fluent (`@fluent/bundle` + `@fluent/react`) rejected**: FTL syntax diverges from ICU, raising friction for community translators (PRD §8) and AI-agent string authoring; runtime parser stays in the bundle; slowest release cadence of the four candidates (no release activity April–May 2026 vs weekly cadence for the alternatives). **`fluent-rs` not applicable**: all localized strings live in the React webview; the Rust core returns structured data, not localized text.
```

### Companion edits required after pasting LD-52

1. **architecture.md:1247** — change `LD-1 through LD-51 + step-6 amendments` to `LD-1 through LD-52 + step-6 amendments`.
2. **architecture.md:1264** — replace the bracketed candidate-list-and-stub paragraph (`2. i18n library selection — required before first translatable UI string lands in shell-ui/. Candidates…`) with: `2. i18n library — selected: Lingui v6.x (LD-52). The "t("key") stub" interim plan is superseded; new UI strings are authored directly with Lingui macros from day 1.`
3. **architecture.md:1314–1316** — drop "i18n library" from the "3 remaining important gaps" sentence (now 2 gaps: PDF crate, PRD reconciliation).
4. **architecture.md:1329** — remove the line `- i18n library selection before v0.1 Alpha UI sprint.`
5. **architecture.md:1354–1355** — remove the entire "i18n library decision: spike react-intl vs lingui vs i18next vs fluent-rs…" follow-up item; the spike is closed by this research document.
6. **architecture.md:1285** — update `LD-1..LD-51 + version policy memory` → `LD-1..LD-52 + version policy memory`.
7. **Stack versions table (around architecture.md:162–187)** — add a `@lingui/*` row pinned to `^6.0.1` (latest stable as of 2026-05-19, per [[feedback_version_policy]]).
8. **First Implementation Stories (architecture.md:342+)** — add to Story 2 dependency install: `@lingui/core @lingui/react @lingui/cli @lingui/vite-plugin @lingui/swc-plugin eslint-plugin-lingui`.

---

## Sources

All sources fetched/verified 2026-05-19 via `ctx7` and GitHub Releases API.

- FormatJS — https://formatjs.github.io/docs/react-intl/upgrade-guide-8.x, https://formatjs.github.io/docs/guides/performance, https://formatjs.github.io/docs/guides/advanced-usage, https://formatjs.github.io/docs/guides/migrate-from-i18next
- Lingui — https://github.com/lingui/js-lingui (README, `installation.mdx`, `vite-plugin.md`, `swc-plugin.md`, `optimizing-bundle-size.md`, `macro.mdx`, `tutorials/react.md`, `releases/migration-4.md`)
- react-i18next — https://github.com/i18next/react-i18next (TypeScript typing config, complete app setup)
- Project Fluent — https://github.com/projectfluent/fluent.js (fluent-bundle README, fluent-react README + CHANGELOG, fluent-syntax README)
- Release cadence — GitHub Releases API (formatjs/formatjs, lingui/js-lingui, i18next/i18next, projectfluent/fluent.js)
- Orgsidian internal — `architecture.md` (LD-1, LD-2, LD-6, LD-18, LD-24, LD-28, LD-29, LD-44, LD-45, FR-10, version policy), `prds/prd-orgsidian-2026-05-19/prd.md` §8 line 524.
