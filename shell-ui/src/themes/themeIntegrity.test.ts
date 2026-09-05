/**
 * Theme-mechanism integrity — Story 6.7 (companion to contrast.test.ts).
 *
 * `contrast.test.ts` gates the `--org-*` palette in tokens.css/light.css/dark.css,
 * but two things it cannot see would ship a broken theme with a green suite:
 *
 *  1. The actual switch mechanism for every shadcn primitive and Tailwind
 *     `dark:` utility lives in `styles/app.css` (the `@custom-variant dark`
 *     definition + the shadcn OKLch dark-token block). If either is reverted to
 *     the old `.dark` class — or drifts out of lockstep with the attribute
 *     `themeMode.ts` writes — the `--org-*` blocks still flip, so contrast.test,
 *     themeMode.test and AppearanceSettings.test all stay green while the
 *     visible chrome stays light. This pins both `app.css` selectors to the
 *     exact `body[data-theme="dark"]` string themeMode.ts targets.
 *
 *  2. `tokens.css`'s `:root` (pre-JS / no-JS default) and `light.css`'s
 *     `body[data-theme="light"]` block are two independent copies of the light
 *     palette. Nothing else asserts they are value-identical, so they could
 *     drift and an explicit "Light" selection would no longer match the
 *     before-JS state. This asserts they declare the same tokens and values.
 */

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const __dirname = dirname(fileURLToPath(import.meta.url));
const read = (rel: string): string => readFileSync(resolve(__dirname, rel), 'utf-8');
const stripComments = (css: string): string => css.replace(/\/\*[\s\S]*?\*\//g, '');

/** Extract `--org-*: value;` declarations from a single selector block. */
function extractOrgVars(css: string, selectorPattern: RegExp): Map<string, string> {
  const match = css.match(selectorPattern);
  if (!match) throw new Error(`selector block not found: ${selectorPattern}`);
  const vars = new Map<string, string>();
  const declRegex = /(--org-[a-z0-9-]+)\s*:\s*([^;]+?)\s*;/g;
  let m: RegExpExecArray | null;
  while ((m = declRegex.exec(match[1])) !== null) {
    vars.set(m[1], m[2].trim());
  }
  return vars;
}

describe('theme switch mechanism — styles/app.css (Story 6.7)', () => {
  const appCss = stripComments(read('../styles/app.css'));

  it('routes Tailwind\'s dark: variant at body[data-theme="dark"]', () => {
    expect(appCss).toMatch(
      /@custom-variant\s+dark\s*\(\s*&:is\(\s*body\[data-theme=["']dark["']\]\s*\*\)\s*\)/,
    );
  });

  it('scopes the shadcn dark token block to body[data-theme="dark"]', () => {
    expect(appCss).toMatch(/body\[data-theme=["']dark["']\]\s*\{/);
  });

  it('leaves no legacy `.dark` class selector wired anywhere in app.css', () => {
    // Comment mentions of `.dark` are stripped above; any remaining `.dark`
    // would be a live selector (block or `:is(.dark *)` variant) — the exact
    // half-wired state Story 6.7 set out to remove.
    expect(appCss).not.toMatch(/\.dark\b/);
  });
});

describe(':root and light.css declare an identical light palette (Story 6.7)', () => {
  const rootVars = extractOrgVars(read('tokens.css'), /:root\s*\{([\s\S]*?)\}/);
  const lightVars = extractOrgVars(
    read('light.css'),
    /body\[data-theme=["']light["']\]\s*\{([\s\S]*?)\}/,
  );

  it('declares the same set of --org-* tokens in both blocks', () => {
    expect([...lightVars.keys()].sort()).toEqual([...rootVars.keys()].sort());
  });

  it('declares identical values for every token (no drift)', () => {
    for (const [name, value] of rootVars) {
      expect(lightVars.get(name), `value mismatch for ${name}`).toBe(value);
    }
  });
});
