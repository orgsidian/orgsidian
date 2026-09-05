/**
 * Contrast-matrix test — LD-58 gate #1 (Story 1.17; extended by Story 6.7).
 *
 * Story 6.7 split the single Story 1.17 `tokens.css` scaffold into the
 * architecture step 3 file layout (`tokens.css` + `light.css` + `dark.css`)
 * and moved the theme-select mechanism from a `.dark` class to
 * `body[data-theme="dark" | "light"]` (see `themeMode.ts`). This test now
 * reads all three files and asserts across three selector blocks:
 * `:root` (tokens.css — pre-JS / structural default), `body[data-theme="light"]`
 * (light.css), `body[data-theme="dark"]` (dark.css).
 *
 * Extracts every (--org-*-fg, --org-*-bg) pair via the explicit `@pair-role:`
 * + `@pair-bg:` annotation convention (unchanged from Story 1.17); computes the
 * WCAG 2.1 relative-luminance contrast ratio `(L1 + 0.05) / (L2 + 0.05)` per
 * pair; asserts:
 *   - body-text  pairs >= 4.5:1 (WCAG 2.1 SC 1.4.3 AA)
 *   - large-text pairs >= 3.0:1 (WCAG 2.1 SC 1.4.3 AA)
 *   - ui-chrome  pairs >= 3.0:1 (WCAG 2.1 SC 1.4.11 AA non-text contrast)
 *
 * Implements NFR-9 / LD-58.
 */

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

type Role = 'body-text' | 'large-text' | 'ui-chrome';

interface Pair {
  fg: string;
  bg: string;
  role: Role;
  fgName: string;
  bgName: string;
}

interface SelectorBlock {
  selector: string;
  pairs: Pair[];
  unpaired: { name: string; reason: 'missing-role' | 'missing-bg' | 'unknown-bg' }[];
}

const ROLE_THRESHOLDS: Record<Role, number> = {
  'body-text': 4.5,
  'large-text': 3.0,
  'ui-chrome': 3.0,
};

// Minimum pair count per selector block; protects against parser regressions
// that would silently yield a vacuous green gate. Bump when palette grows.
const MIN_PAIRS_PER_BLOCK = 5;

// Expected selector blocks post-Story-6.7 split: tokens.css's structural
// `:root` default, plus the explicit light + dark theme files.
const EXPECTED_SELECTORS = [':root', 'body[data-theme="light"]', 'body[data-theme="dark"]'];

const __dirname = dirname(fileURLToPath(import.meta.url));
// Concatenated so `parseTokens` can locate all three selector blocks
// regardless of which physical file declares them.
const TOKENS_CSS = [
  readFileSync(resolve(__dirname, 'tokens.css'), 'utf-8'),
  readFileSync(resolve(__dirname, 'light.css'), 'utf-8'),
  readFileSync(resolve(__dirname, 'dark.css'), 'utf-8'),
].join('\n');

export function parseTokens(css: string): SelectorBlock[] {
  const blocks: SelectorBlock[] = [];
  const selectors: { selector: string; pattern: RegExp }[] = [
    { selector: ':root', pattern: /:root\s*\{([^}]*)\}/ },
    {
      selector: 'body[data-theme="light"]',
      pattern: /body\[data-theme=["']light["']\]\s*\{([^}]*)\}/,
    },
    {
      selector: 'body[data-theme="dark"]',
      pattern: /body\[data-theme=["']dark["']\]\s*\{([^}]*)\}/,
    },
  ];

  const roleRegex = /^\s*\/\*\s*@pair-role:\s*(body-text|large-text|ui-chrome)\s*\*\/\s*$/;
  const pairBgRegex = /^\s*\/\*\s*@pair-bg:\s*(--org-bg-[a-z0-9-]+)\s*\*\/\s*$/;
  const declRegex = /^\s*(--org-[a-z0-9-]+)\s*:\s*([^;]+?)\s*;/;

  for (const { selector, pattern } of selectors) {
    const match = css.match(pattern);
    if (!match) continue;
    const body = match[1];
    const lines = body.split('\n');
    const pairs: Pair[] = [];
    const unpaired: SelectorBlock['unpaired'] = [];
    const bgValues = new Map<string, string>();
    let pendingRole: Role | null = null;
    let pendingBgName: string | null = null;

    for (const line of lines) {
      const roleMatch = line.match(roleRegex);
      if (roleMatch) {
        pendingRole = roleMatch[1] as Role;
        continue;
      }
      const pairBgMatch = line.match(pairBgRegex);
      if (pairBgMatch) {
        pendingBgName = pairBgMatch[1];
        continue;
      }
      const declMatch = line.match(declRegex);
      if (!declMatch) continue;
      const name = declMatch[1];
      const value = declMatch[2];
      if (name.startsWith('--org-bg-')) {
        bgValues.set(name, value);
        // backgrounds carry no pair metadata; drop any pending annotations
        // (defensive — stacked stray @pair-* before a bg would otherwise leak).
        pendingRole = null;
        pendingBgName = null;
        continue;
      }
      // Any non-bg --org-* token MUST carry both @pair-role and @pair-bg.
      if (pendingRole === null) {
        unpaired.push({ name, reason: 'missing-role' });
        pendingBgName = null;
        continue;
      }
      if (pendingBgName === null) {
        unpaired.push({ name, reason: 'missing-bg' });
        pendingRole = null;
        continue;
      }
      const bgValue = bgValues.get(pendingBgName);
      if (bgValue === undefined) {
        unpaired.push({ name, reason: 'unknown-bg' });
        pendingRole = null;
        pendingBgName = null;
        continue;
      }
      pairs.push({
        fg: value,
        bg: bgValue,
        role: pendingRole,
        fgName: name,
        bgName: pendingBgName,
      });
      pendingRole = null;
      pendingBgName = null;
    }
    blocks.push({ selector, pairs, unpaired });
  }
  return blocks;
}

export function relativeLuminance(rgb: { r: number; g: number; b: number }): number {
  const channel = (c: number): number => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  const R = channel(rgb.r);
  const G = channel(rgb.g);
  const B = channel(rgb.b);
  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

function parseHex(color: string): { r: number; g: number; b: number } {
  const trimmed = color.trim();
  if (trimmed.startsWith('oklch(') || trimmed.startsWith('oklab(')) {
    throw new Error(
      `[contrast.test] OKLch/OKLab color space not supported (got ${trimmed}). ` +
        'Story 6.7 kept the --org-* palette hex-only; a future story adds a ' +
        'color-space-conversion utility if the palette adopts OKLch.',
    );
  }
  const m = trimmed.match(/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/);
  if (!m) {
    throw new Error(`[contrast.test] unsupported color literal: ${trimmed}`);
  }
  let hex = m[1];
  if (hex.length === 3) {
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('');
  }
  return {
    r: parseInt(hex.slice(0, 2), 16),
    g: parseInt(hex.slice(2, 4), 16),
    b: parseInt(hex.slice(4, 6), 16),
  };
}

export function contrastRatio(fg: string, bg: string): number {
  const L1 = relativeLuminance(parseHex(fg));
  const L2 = relativeLuminance(parseHex(bg));
  const [lighter, darker] = L1 >= L2 ? [L1, L2] : [L2, L1];
  return (lighter + 0.05) / (darker + 0.05);
}

describe('LD-58 contrast gate — tokens.css + light.css + dark.css (Story 6.7)', () => {
  const blocks = parseTokens(TOKENS_CSS);

  it('parses exactly 3 selector blocks (:root + light + dark)', () => {
    expect(blocks.map((b) => b.selector).sort()).toEqual([...EXPECTED_SELECTORS].sort());
    expect(blocks).toHaveLength(3);
  });

  it('every non-bg --org-* token carries @pair-role + @pair-bg (no unpaired)', () => {
    const unpaired = blocks.flatMap((b) =>
      b.unpaired.map((u) => `${b.selector} ${u.name} [${u.reason}]`),
    );
    expect(unpaired).toEqual([]);
  });

  it(`each block has >= ${MIN_PAIRS_PER_BLOCK} fg pairs (vacuous-gate floor)`, () => {
    for (const b of blocks) {
      expect(b.pairs.length).toBeGreaterThanOrEqual(MIN_PAIRS_PER_BLOCK);
    }
  });

  for (const block of blocks) {
    for (const pair of block.pairs) {
      it(`${block.selector} ${pair.fgName} on ${pair.bgName} (${pair.role}) meets WCAG floor`, () => {
        const ratio = contrastRatio(pair.fg, pair.bg);
        expect(ratio).toBeGreaterThanOrEqual(ROLE_THRESHOLDS[pair.role]);
      });
    }
  }

  it('relativeLuminance white === 1.0', () => {
    expect(relativeLuminance({ r: 255, g: 255, b: 255 })).toBeCloseTo(1.0, 10);
  });

  it('relativeLuminance black === 0.0', () => {
    expect(relativeLuminance({ r: 0, g: 0, b: 0 })).toBeCloseTo(0.0, 10);
  });

  it('contrastRatio(#000, #fff) === 21.0', () => {
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21.0, 10);
  });
});
