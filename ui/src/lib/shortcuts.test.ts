import { describe, expect, it } from 'vitest';
import { comboFromEvent } from './shortcuts';

const event = (
  code: string,
  mods: Partial<Record<'ctrl' | 'alt' | 'shift' | 'meta', boolean>> = {},
) => ({
  code,
  ctrlKey: !!mods.ctrl,
  altKey: !!mods.alt,
  shiftKey: !!mods.shift,
  metaKey: !!mods.meta,
});

describe('comboFromEvent (HK-002)', () => {
  it('builds the canonical text with modifiers in a fixed order', () => {
    expect(comboFromEvent(event('KeyV', { meta: true, ctrl: true }))).toBe('Ctrl+Win+V');
    expect(comboFromEvent(event('Digit1', { shift: true, ctrl: true }))).toBe('Ctrl+Shift+1');
    expect(comboFromEvent(event('KeyA', { alt: true, shift: true, ctrl: true, meta: true }))).toBe(
      'Ctrl+Alt+Shift+Win+A',
    );
  });

  it('uses physical keys, independent of the keyboard layout', () => {
    expect(comboFromEvent(event('Numpad4', { ctrl: true }))).toBe('Ctrl+Num4');
    expect(comboFromEvent(event('F13'))).toBe('F13');
    expect(comboFromEvent(event('ArrowUp', { alt: true }))).toBe('Alt+Up');
    expect(comboFromEvent(event('Semicolon', { ctrl: true }))).toBe('Ctrl+Semicolon');
    expect(comboFromEvent(event('NumpadEnter', { ctrl: true }))).toBe('Ctrl+Enter');
  });

  it('waits while only modifiers are pressed and ignores unknown keys', () => {
    expect(comboFromEvent(event('ShiftLeft', { shift: true }))).toBeNull();
    expect(comboFromEvent(event('ControlRight', { ctrl: true }))).toBeNull();
    expect(comboFromEvent(event('MetaLeft', { meta: true }))).toBeNull();
    expect(comboFromEvent(event('LaunchMail', { ctrl: true }))).toBeNull();
  });
});
