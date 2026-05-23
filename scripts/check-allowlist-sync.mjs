#!/usr/bin/env node
// scripts/check-allowlist-sync.mjs — Story 1.8 AC6 cross-tool allowlist sync.
// Closes Story 1.7 deferred-work "Lockstep cargo↔JS allowlists not enforced".
// Asserts every SPDX in the symmetric difference between deny.toml
// [licenses].allow and check-pnpm-licenses.mjs ALLOWLIST is documented in
// docs/security/advisory-exceptions.md "License exceptions" table.
// Regex-extracts both arrays (no toml-parse dep — Story 1.7 AC9 budget
// discipline). Brittle on reformat; baseline-count self-checks catch drift.

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
  // Strip `#`/`//` line-comments before scanning so quoted strings inside
  // comments (e.g. doc-prose like `"License exceptions"`) don't pollute.
  const stripped = blob.replace(/^[ \t]*(#|\/\/).*$/gm, "");
  return new Set([...stripped.matchAll(/"([^"]+)"/g)].map((m) => m[1]));
}

// deny.toml — only one `allow = [...]` array in the file (under [licenses]).
const denyMatch = read("deny.toml").match(/^\s*allow\s*=\s*\[([\s\S]*?)\]/m);
const cargoAllow = extractQuoted(denyMatch?.[1], "`allow = [...]` in deny.toml");

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
