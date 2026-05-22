#!/usr/bin/env node
// scripts/check-pnpm-licenses.mjs — Story 1.7 (LD-37) JS-side license filter.
//
// Reads `pnpm licenses ls --prod --long --json` from stdin, validates every
// prod-dep's license against the LD-37 allowlist (same set as deny.toml).
// Exits 0 on clean; exits 1 with a per-package report on any rejection.
//
// SPDX-expression handling: an expression like "(MIT OR Apache-2.0)" passes
// if ANY alternative is on the allowlist (user-friendly interpretation —
// the dual-licensing intent is that consumers may pick a compatible branch).
// Conjunctions ("MIT AND Apache-2.0") require ALL components on the
// allowlist (every component constrains the consumer).

const ALLOWLIST = new Set([
  "MIT",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unlicense",
  "Zlib",
  "MPL-2.0",
  // Transitive-forced additions for the pnpm side (Story 1.7).
  // Recorded in docs/security/advisory-exceptions.md.
  "0BSD",       // tslib — OSI-approved zero-clause BSD (≈ public domain).
  "CC-BY-4.0",  // caniuse-lite — browser-data tables (data, not code).
]);

function readStdin() {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => (data += chunk));
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", reject);
  });
}

function isAllowed(spdx) {
  if (!spdx || spdx === "UNKNOWN") return false;
  const clean = spdx.replace(/[()]/g, "").trim();
  if (ALLOWLIST.has(clean)) return true;
  if (/\bAND\b/i.test(clean)) {
    return clean.split(/\s+AND\s+/i).every((s) => ALLOWLIST.has(s.trim()));
  }
  if (/\bOR\b/i.test(clean)) {
    return clean.split(/\s+OR\s+/i).some((s) => ALLOWLIST.has(s.trim()));
  }
  return false;
}

function collectPackages(json) {
  const parsed = JSON.parse(json);
  const out = [];
  for (const [license, entries] of Object.entries(parsed)) {
    if (!Array.isArray(entries)) continue;
    for (const entry of entries) {
      out.push({
        name: entry.name ?? "<unknown>",
        version: entry.version ?? "<unknown>",
        license,
      });
    }
  }
  return out;
}

const stdin = await readStdin();
if (!stdin.trim()) {
  console.error("check-pnpm-licenses: empty stdin (expected `pnpm licenses ls --prod --long --json`).");
  process.exit(1);
}

const offenders = collectPackages(stdin).filter((p) => !isAllowed(p.license));
if (offenders.length === 0) {
  console.log(`check-pnpm-licenses: OK — every prod-dep license is on the LD-37 allowlist.`);
  process.exit(0);
}

console.error(`check-pnpm-licenses: FAIL — ${offenders.length} package(s) outside the LD-37 allowlist:`);
for (const p of offenders) {
  console.error(`  - ${p.name}@${p.version} (license: ${p.license})`);
}
console.error("\nAdd an explicit exception to docs/security/advisory-exceptions.md and decide whether the package can be replaced before silencing.");
process.exit(1);
