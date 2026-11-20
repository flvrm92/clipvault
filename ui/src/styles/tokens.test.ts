import { describe, expect, it } from 'vitest';
import { SURFACES } from '../lib/appearance';
import { contrastRatio } from '../lib/color';
import css from './tokens.css?raw';

/** Custom properties declared in the first rule whose selector is exactly `selector`. */
function tokens(selector: string): Record<string, string> {
  const start = css.indexOf(`${selector} {`);
  if (start < 0) throw new Error(`selector not found: ${selector}`);
  const body = css.slice(css.indexOf('{', start) + 1, css.indexOf('}', start));
  return Object.fromEntries(
    [...body.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2]!.trim().toLowerCase()]),
  );
}

/** Resolves `var(--name)` aliases and keeps only hex colors. */
function resolve(declared: Record<string, string>): Record<string, string> {
  const value = (v: string, depth = 0): string => {
    const alias = /^var\((--[\w-]+)\)$/.exec(v);
    return alias && depth < 5 ? value(declared[alias[1]!] ?? '', depth + 1) : v;
  };
  return Object.fromEntries(
    Object.entries(declared)
      .map(([k, v]) => [k, value(v)] as const)
      .filter(([, v]) => /^#[0-9a-f]{3,6}$/.test(v)),
  );
}

const light = resolve(tokens(':root'));
const dark = resolve({ ...tokens(':root'), ...tokens(":root[data-theme='dark']") });
const themes = { light, dark } as const;

const SURFACE_TOKENS = ['--surface', '--surface-raised', '--surface-hover', '--surface-selected'];
const TEXT_TOKENS = [
  '--text-primary',
  '--text-secondary',
  '--accent',
  '--success',
  '--warning',
  '--danger',
];
const COMPONENT_TOKENS = ['--border-strong', '--focus-ring'];

describe.each(Object.entries(themes))('%s theme tokens', (_name, t) => {
  const ratio = (fg: string, bg: string) => contrastRatio(t[fg]!, t[bg]!);

  it('declares every checked token as a color', () => {
    for (const name of ['--inverse-surface', '--inverse-text', '--inverse-accent']) {
      expect(t[name], name).toMatch(/^#/);
    }
  });

  it.each(TEXT_TOKENS.flatMap((fg) => SURFACE_TOKENS.map((bg) => [fg, bg])))(
    '[A11Y-002] text %s on %s reaches 4.5:1',
    (fg, bg) => expect(ratio(fg!, bg!)).toBeGreaterThanOrEqual(4.5),
  );

  it.each([
    ['--accent-text', '--accent'],
    ['--accent-text', '--danger'],
    ['--text-primary', '--warning-surface'],
    ['--warning', '--warning-surface'],
    ['--accent', '--warning-surface'],
    ['--inverse-text', '--inverse-surface'],
    ['--inverse-accent', '--inverse-surface'],
  ])('[A11Y-002] text %s on %s reaches 4.5:1', (fg, bg) =>
    expect(ratio(fg!, bg!)).toBeGreaterThanOrEqual(4.5),
  );

  it.each(
    COMPONENT_TOKENS.flatMap((fg) =>
      ['--surface', '--surface-raised', '--surface-hover'].map((bg) => [fg, bg]),
    ),
  )('[A11Y-002] component %s on %s reaches 3:1', (fg, bg) =>
    expect(ratio(fg!, bg!)).toBeGreaterThanOrEqual(3),
  );
});

describe('token consistency', () => {
  it('the dark theme is the same whether chosen by the OS or by the user', () => {
    expect(tokens(":root:not([data-theme='light'])")).toEqual(tokens(":root[data-theme='dark']"));
  });

  it('[THM-004] the accent fallback checks the real surfaces', () => {
    expect(SURFACES.light).toEqual([light['--surface'], light['--surface-raised']]);
    expect(SURFACES.dark).toEqual([dark['--surface'], dark['--surface-raised']]);
  });

  it('[A11Y-004] [A11Y-006] respects reduced motion and forced colors', () => {
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)[^}]*animation: none/);
    expect(css).toContain('@media (forced-colors: active)');
  });
});
