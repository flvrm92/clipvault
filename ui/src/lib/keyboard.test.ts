import { describe, expect, it } from 'vitest';
import { resolveKey, type KeyContext } from './keyboard';

const idle: KeyContext = { searchEmpty: true, inputHasSelection: false, caretAtEnd: true };
const typing: KeyContext = { searchEmpty: false, inputHasSelection: false, caretAtEnd: false };
const key = (k: string, mods: Partial<{ shift: boolean; ctrl: boolean; alt: boolean }> = {}) => ({
  key: k,
  shift: false,
  ctrl: false,
  alt: false,
  ...mods,
});

describe('resolveKey (specs/06 §2.2)', () => {
  it('[A11Y-001] arrows and page keys move the selection', () => {
    expect(resolveKey(key('ArrowDown'), typing)).toEqual({ type: 'move', by: 1 });
    expect(resolveKey(key('ArrowUp'), typing)).toEqual({ type: 'move', by: -1 });
    expect(resolveKey(key('PageDown'), typing)).toEqual({ type: 'page', direction: 1 });
    expect(resolveKey(key('PageUp'), typing)).toEqual({ type: 'page', direction: -1 });
  });

  it('[A11Y-001] Home/End navigate only while the search is empty', () => {
    expect(resolveKey(key('Home'), idle)).toEqual({ type: 'edge', edge: 'first' });
    expect(resolveKey(key('End'), idle)).toEqual({ type: 'edge', edge: 'last' });
    expect(resolveKey(key('Home'), typing)).toBeNull();
  });

  it('[PST-001] [PST-002] Enter pastes, Shift+Enter pastes trimmed text', () => {
    expect(resolveKey(key('Enter'), typing)).toEqual({ type: 'paste', mode: 'asIs' });
    expect(resolveKey(key('Enter', { shift: true }), typing)).toEqual({
      type: 'paste',
      mode: 'trimmed',
    });
  });

  it('[PST-004] Alt+1..9 paste the nth visible item', () => {
    expect(resolveKey(key('1', { alt: true }), typing)).toEqual({ type: 'pasteNth', index: 0 });
    expect(resolveKey(key('9', { alt: true }), typing)).toEqual({ type: 'pasteNth', index: 8 });
    expect(resolveKey(key('0', { alt: true }), typing)).toBeNull();
  });

  it('[HIST-011] Ctrl+C copies the item unless text is selected in the search box', () => {
    expect(resolveKey(key('c', { ctrl: true }), idle)).toEqual({ type: 'copy' });
    expect(resolveKey(key('c', { ctrl: true }), { ...typing, inputHasSelection: true })).toBeNull();
  });

  it('[HIST-005] Ctrl+P toggles the pin', () => {
    expect(resolveKey(key('p', { ctrl: true }), typing)).toEqual({ type: 'togglePin' });
  });

  it('[HIST-006] Delete removes the item unless it would delete search text', () => {
    expect(resolveKey(key('Delete'), idle)).toEqual({ type: 'delete' });
    expect(resolveKey(key('Delete'), { ...typing, caretAtEnd: true })).toEqual({ type: 'delete' });
    expect(resolveKey(key('Delete'), typing)).toBeNull();
  });

  it('[HIST-008] Space previews only while the search is empty', () => {
    expect(resolveKey(key(' '), idle)).toEqual({ type: 'preview' });
    expect(resolveKey(key(' '), typing)).toBeNull();
  });

  it('[HIST-004] Tab and Shift+Tab cycle the filters', () => {
    expect(resolveKey(key('Tab'), typing)).toEqual({ type: 'filter', direction: 1 });
    expect(resolveKey(key('Tab', { shift: true }), typing)).toEqual({
      type: 'filter',
      direction: -1,
    });
  });

  it('[PST-007] Escape clears the search first, then closes', () => {
    expect(resolveKey(key('Escape'), typing)).toEqual({ type: 'clearSearch' });
    expect(resolveKey(key('Escape'), idle)).toEqual({ type: 'close' });
  });

  it('[CFG-001] Ctrl+, opens the settings', () => {
    expect(resolveKey(key(',', { ctrl: true }), typing)).toEqual({ type: 'settings' });
  });

  it('leaves ordinary typing to the search box', () => {
    expect(resolveKey(key('a'), typing)).toBeNull();
    expect(resolveKey(key('Backspace'), typing)).toBeNull();
  });
});
