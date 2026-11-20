import { afterEach, describe, expect, it } from 'vitest';
import { applyAppearance } from './appearance';

const root = document.documentElement;

describe('applyAppearance', () => {
  afterEach(() => root.removeAttribute('style'));

  it('[THM-001] [THM-002] sets theme and backdrop on the root element', () => {
    applyAppearance({ theme: 'dark', backdrop: 'acrylic', accent: null });

    expect(root.dataset.theme).toBe('dark');
    expect(root.dataset.backdrop).toBe('acrylic');
    expect(root.style.getPropertyValue('--accent')).toBe('');
  });

  it('[THM-004] uses the Windows accent with a readable text color', () => {
    applyAppearance({ theme: 'light', backdrop: 'solid', accent: '#005a9e' });

    expect(root.style.getPropertyValue('--accent')).toBe('#005a9e');
    expect(root.style.getPropertyValue('--focus-ring')).toBe('#005a9e');
    expect(root.style.getPropertyValue('--accent-text')).toBe('#ffffff');
  });

  it('[THM-004] [A11Y-002] keeps the brand color when the accent lacks 3:1 contrast', () => {
    applyAppearance({ theme: 'light', backdrop: 'solid', accent: '#a6d8ff' });

    expect(root.style.getPropertyValue('--accent')).toBe('');
  });

  it('[THM-004] turning the accent off restores the brand color', () => {
    applyAppearance({ theme: 'dark', backdrop: 'solid', accent: '#a6d8ff' });
    expect(root.style.getPropertyValue('--accent')).toBe('#a6d8ff');

    applyAppearance({ theme: 'dark', backdrop: 'solid', accent: null });
    expect(root.style.getPropertyValue('--accent')).toBe('');
  });
});
