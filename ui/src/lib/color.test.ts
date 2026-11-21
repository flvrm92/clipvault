import { describe, expect, it } from 'vitest';
import { contrastRatio, parseHex, readableTextOn } from './color';

describe('color', () => {
  it('parses #rrggbb and #rgb', () => {
    expect(parseHex('#4b3fd1')).toEqual([0x4b, 0x3f, 0xd1]);
    expect(parseHex('#fff')).toEqual([255, 255, 255]);
    expect(parseHex('red')).toBeNull();
  });

  it('[A11Y-002] computes WCAG contrast ratios', () => {
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 1);
    expect(contrastRatio('#ffffff', '#ffffff')).toBeCloseTo(1, 5);
    expect(contrastRatio('#777777', '#ffffff')).toBeCloseTo(4.48, 2);
    expect(contrastRatio('nope', '#ffffff')).toBe(1);
  });

  it('[THM-004] picks the text color with the highest contrast', () => {
    expect(readableTextOn('#005a9e')).toBe('#ffffff');
    expect(readableTextOn('#a6d8ff')).toBe('#14131f');
  });
});
