#!/usr/bin/env node
// scripts/check-allowlist-sync.mjs — Story 1.8 AC6 cross-tool allowlist sync.
// Closes Story 1.7 deferred-work "Lockstep cargo↔JS allowlists not enforced".
// Asserts every SPDX in the symmetric difference between deny.toml
// [licenses].allow and check-pnpm-licenses.mjs ALLOWLIST is documented in
// docs/security/advisory-exceptions.md "License exceptions" table.
// Regex-extracts both arrays (no toml-parse dep — Story 1.7 AC9 budget
// discipline). Brittle on reformat; baseline-count self-checks catch drift.
//
// Section invariants this script depends on (document and verify each):
// - deny.toml has a `[licenses]` header containing exactly one `allow = [...]`
//   array (other sections may also use `allow`, hence section-scoping).
// - check-pnpm-licenses.mjs has exactly one `const ALLOWLIST = new Set([...])`.
// - advisory-exceptions.md "### License exceptions" table cell-1 uses a
//   SINGLE backtick-quoted SPDX (e.g. `` `Apache-2.0 WITH LLVM-exception` ``).
//   Split-quoting (`` `Apache-2.0` WITH `LLVM-exception` ``) is NOT supported
//   and will trigger a false divergence — keep cell-1 single-span.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(resolve(repo, p), "utf8");

function extractQuoted(blob, label) {
  if (!blob) {
    console.error(`check-allowlist-sync: FAIL — could not locate ${label}.`);
    process.exit(1);
  }
  // Strip both line-leading AND inline trailing `#` / `//` comments — without
  // the inline strip, a trailing comment like `"MIT", # used by "foo"` leaks
  // the inner `"foo"` into the extracted set.
  const stripped = blob.replace(/(#|\/\/)[^\n]*/g, "");
  return new Set([...stripped.matchAll(/"([^"]+)"/g)].map((m) => m[1]));
}

// deny.toml — extract the `allow = [...]` array scoped to the `[licenses]`
// section. Other sections (`[bans]`, `[sources]`) may also legitimately use
// `allow` — a non-scoped match would silently validate the wrong list.
const denyToml = read("deny.toml");
// `(?=^\[|$)` with `m` flag matches the next section header OR end-of-string.
// JS regex has no `\Z`; emulate via end-of-string lookahead at file boundary.
const licensesSection = denyToml.match(/^\[licenses\][\s\S]*?(?=\n\[|$(?![\s\S]))/m);
const denyMatch = licensesSection?.[0]?.match(/^\s*allow\s*=\s*\[([\s\S]*?)\]/m);
const cargoAllow = extractQuoted(denyMatch?.[1], "`[licenses].allow = [...]` in deny.toml");

// check-pnpm-licenses.mjs — single ALLOWLIST = new Set([...]) literal.
const pnpmMatch = read("scripts/check-pnpm-licenses.mjs")
  .match(/const ALLOWLIST\s*=\s*new Set\(\s*\[([\s\S]*?)\]\s*\)/);
const pnpmAllow = extractQuoted(pnpmMatch?.[1], "ALLOWLIST in check-pnpm-licenses.mjs");

// Baseline self-check: cargo>=11, pnpm>=10 (Story 1.7 baseline). Drift past
// these counts means the regex missed entries — hard fail with diagnostic.
for (const [set, n, name] of [[cargoAllow, 11, "deny.toml"], [pnpmAllow, 10, "check-pnpm-licenses.mjs"]]) {
  if (set.size < n) {
    console.error(`check-allowlist-sync: FAIL — extracted ${set.size}<${n} entries from ${name}. File reformat? Update regex.`);
    process.exit(1);
  }
}

const symmDiff = [
  ...[...cargoAllow].filter((x) => !pnpmAllow.has(x)),
  ...[...pnpmAllow].filter((x) => !cargoAllow.has(x)),
].sort();

// Ledger: SPDX IDs appear in the first table-cell, backtick-quoted, under
// the "### License exceptions" subsection of the ledger.
const ledger = read("docs/security/advisory-exceptions.md");
const section = ledger.match(/### License exceptions[\s\S]*?(?=\n### |\n## |$)/);
if (!section) {
  console.error("check-allowlist-sync: FAIL — could not locate '### License exceptions' section.");
  process.exit(1);
}
const documented = new Set([...section[0].matchAll(/\|\s*`([^`]+)`/g)].map((m) => m[1].trim()));

const undocumented = symmDiff.filter((id) => !documented.has(id));
if (undocumented.length === 0) {
  console.log(`check-allowlist-sync: OK — ${symmDiff.length} divergent SPDX fully documented.`);
  process.exit(0);
}

console.error(`check-allowlist-sync: FAIL — ${undocumented.length} undocumented divergence(s):`);
for (const id of undocumented) {
  console.error(`  - ${id} (${cargoAllow.has(id) ? "cargo-only" : "pnpm-only"})`);
}
console.error("\nAdd a row to docs/security/advisory-exceptions.md 'License exceptions' table OR remove the SPDX from the unilateral allowlist.");
process.exit(1);
