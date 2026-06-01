/**
 * Contrast-matrix test — LD-58 gate #1 (Story 1.17).
 *
 * Extracts every (--org-*-fg, --org-*-bg) pair from tokens.css per the
 * `@pair-role:` comment convention; computes the WCAG 2.1 relative-luminance
 * contrast ratio `(L1 + 0.05) / (L2 + 0.05)` per pair; asserts:
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
}

const ROLE_THRESHOLDS: Record<Role, number> = {
  'body-text': 4.5,
  'large-text': 3.0,
  'ui-chrome': 3.0,
};

const __dirname = dirname(fileURLToPath(import.meta.url));
const TOKENS_CSS = readFileSync(resolve(__dirname, 'tokens.css'), 'utf-8');

export function parseTokens(css: string): SelectorBlock[] {
  const blocks: SelectorBlock[] = [];
  const selectors: { selector: string; pattern: RegExp }[] = [
    { selector: ':root', pattern: /:root\s*\{([^}]*)\}/ },
    { selector: '.dark', pattern: /\.dark\s*\{([^}]*)\}/ },
  ];

  for (const { selector, pattern } of selectors) {
    const match = css.match(pattern);
    if (!match) continue;
    const body = match[1];
    const lines = body.split('\n');
    const pairs: Pair[] = [];
    let pendingRole: Role | null = null;
    let lastBgName: string | null = null;
    let lastBgValue: string | null = null;

    const roleRegex = /^\s*\/\*\s*@pair-role:\s*(body-text|large-text|ui-chrome)\s*\*\/\s*$/;
    const declRegex = /^\s*(--org-[a-z0-9-]+)\s*:\s*([^;]+?)\s*;/;

    for (const line of lines) {
      const roleMatch = line.match(roleRegex);
      if (roleMatch) {
        pendingRole = roleMatch[1] as Role;
        continue;
      }
      const declMatch = line.match(declRegex);
      if (!declMatch) continue;
      const name = declMatch[1];
      const value = declMatch[2];
      if (name.startsWith('--org-bg-')) {
        lastBgName = name;
        lastBgValue = value;
        // backgrounds should not carry a pending role; drop it.
        pendingRole = null;
        continue;
      }
      // Any non-bg --org-* token consumes a pending @pair-role if present.
      if (pendingRole !== null) {
        if (lastBgValue === null || lastBgName === null) {
          throw new Error(
            `[contrast.test] foreground token ${name} declared before any --org-bg-* token in ${selector}`,
          );
        }
        pairs.push({
          fg: value,
          bg: lastBgValue,
          role: pendingRole,
          fgName: name,
          bgName: lastBgName,
        });
        pendingRole = null;
      }
    }
    blocks.push({ selector, pairs });
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
      `[contrast.test] OKLch/OKLab color space not supported in Story 1.17 (got ${trimmed}). Story 6.7 will add a color-space-conversion utility.`,
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

function findUnpaired(css: string): string[] {
  const unpaired: string[] = [];
  const selectors = [
    { selector: ':root', pattern: /:root\s*\{([^}]*)\}/ },
    { selector: '.dark', pattern: /\.dark\s*\{([^}]*)\}/ },
  ];
  const roleRegex = /^\s*\/\*\s*@pair-role:\s*(body-text|large-text|ui-chrome)\s*\*\/\s*$/;
  const declRegex = /^\s*(--org-[a-z0-9-]+)\s*:/;
  const fgRegex = /--org-fg-|--org-.*-fg($|:)/;

  for (const { selector, pattern } of selectors) {
    const match = css.match(pattern);
    if (!match) continue;
    const lines = match[1].split('\n');
    let pendingRole: Role | null = null;
    for (const line of lines) {
      if (roleRegex.test(line)) {
        pendingRole = (line.match(roleRegex) as RegExpMatchArray)[1] as Role;
        continue;
      }
      const dMatch = line.match(declRegex);
      if (!dMatch) continue;
      const name = dMatch[1];
      if (name.startsWith('--org-bg-')) {
        pendingRole = null;
        continue;
      }
      // foreground-like tokens require a pending role
      if (fgRegex.test(name)) {
        if (pendingRole === null) {
          unpaired.push(`${selector} ${name}`);
        }
        pendingRole = null;
      }
    }
  }
  return unpaired;
}

describe('LD-58 contrast gate — tokens.css', () => {
  const blocks = parseTokens(TOKENS_CSS);

  it('parses exactly 2 selector blocks (:root + .dark)', () => {
    expect(blocks.map((b) => b.selector).sort()).toEqual(['.dark', ':root'].sort());
    expect(blocks).toHaveLength(2);
  });

  it('every foreground token has a @pair-role annotation', () => {
    const unpaired = findUnpaired(TOKENS_CSS);
    expect(unpaired).toEqual([]);
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
