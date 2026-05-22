#!/usr/bin/env node
// scripts/check-pnpm-licenses.mjs — Story 1.7 (LD-37) JS-side license filter.
//
// Reads `pnpm licenses ls --prod --long --json` from stdin, validates every
// prod-dep's license against the LD-37 allowlist (same set as deny.toml).
// Exits 0 on clean; exits 1 with a per-package report on any rejection.
//
// SPDX-2 precedence is respected: parens > AND > OR. `WITH <X>-exception`
// suffixes are stripped before lookup — LD-37 policy: `Apache-2.0 WITH
// LLVM-exception` is allowed by virtue of `Apache-2.0` being on the list.

const ALLOWLIST = new Set([
  "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause",
  "ISC", "Unlicense", "Zlib", "MPL-2.0",
  // Transitive-forced additions for the pnpm side (Story 1.7).
  // Recorded in docs/security/advisory-exceptions.md.
  "0BSD",       // tslib — OSI-approved zero-clause BSD.
  "CC-BY-4.0",  // caniuse-lite — browser-data tables (data, not code).
]);

const readStdin = () => new Promise((resolve, reject) => {
  let data = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (c) => (data += c));
  process.stdin.on("end", () => resolve(data));
  process.stdin.on("error", reject);
});

function stripOuterParens(s) {
  while (s.startsWith("(") && s.endsWith(")")) {
    let depth = 0, wraps = true;
    for (let i = 0; i < s.length - 1; i++) {
      if (s[i] === "(") depth++;
      else if (s[i] === ")") { depth--; if (depth === 0) { wraps = false; break; } }
    }
    if (!wraps) break;
    s = s.slice(1, -1).trim();
  }
  return s;
}

function findTopLevel(expr, op) {
  let depth = 0;
  for (let i = 0; i <= expr.length - op.length; i++) {
    if (expr[i] === "(") { depth++; continue; }
    if (expr[i] === ")") { depth--; continue; }
    if (depth !== 0) continue;
    if (expr.slice(i, i + op.length).toUpperCase() !== op) continue;
    const prev = expr[i - 1] ?? " ", next = expr[i + op.length] ?? " ";
    if (/\s/.test(prev) && /\s/.test(next)) return i;
  }
  return -1;
}

function evalSpdx(expr) {
  expr = stripOuterParens(expr.trim());
  const or = findTopLevel(expr, "OR");
  if (or !== -1) return evalSpdx(expr.slice(0, or)) || evalSpdx(expr.slice(or + 2));
  const and = findTopLevel(expr, "AND");
  if (and !== -1) return evalSpdx(expr.slice(0, and)) && evalSpdx(expr.slice(and + 3));
  return ALLOWLIST.has(expr.replace(/\s+WITH\s+\S+-exception\s*$/i, "").trim());
}

const isAllowed = (spdx) =>
  !!spdx && !/^unknown$/i.test(spdx.trim()) && evalSpdx(spdx);

function collectPackages(json) {
  const out = [];
  for (const [license, entries] of Object.entries(JSON.parse(json))) {
    if (!Array.isArray(entries)) continue;
    for (const e of entries) {
      const version = Array.isArray(e.versions)
        ? e.versions.join(", ")
        : (e.version ?? "<unknown>");
      out.push({ name: e.name ?? "<unknown>", version, license });
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
for (const p of offenders) console.error(`  - ${p.name}@${p.version} (license: ${p.license})`);
console.error("\nAdd an explicit exception to docs/security/advisory-exceptions.md and decide whether the package can be replaced before silencing.");
process.exit(1);
