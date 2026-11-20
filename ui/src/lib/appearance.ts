/** Applies the appearance computed by the backend to the document (THM-001..004). */
import type { AppearanceDto } from '../api/bindings/AppearanceDto';
import { contrastRatio, readableTextOn } from './color';

/** Surfaces the accent is drawn on, per theme — mirrors `styles/tokens.css`. */
export const SURFACES = {
  light: ['#f7f7fa', '#ffffff'],
  dark: ['#1c1b22', '#26252e'],
} as const;

/** Minimum contrast of a UI component against its surface (WCAG 1.4.11, A11Y-002). */
const COMPONENT_CONTRAST = 3;

const ACCENT_PROPERTIES = ['--accent', '--focus-ring', '--accent-text'] as const;

export function applyAppearance(
  appearance: AppearanceDto,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.theme = appearance.theme;
  root.dataset.backdrop = appearance.backdrop;
  const accent = appearance.accent;
  const readable =
    !!accent &&
    SURFACES[appearance.theme].every((s) => contrastRatio(accent, s) >= COMPONENT_CONTRAST);
  if (!accent || !readable) {
    ACCENT_PROPERTIES.forEach((p) => root.style.removeProperty(p));
    return;
  }
  root.style.setProperty('--accent', accent);
  root.style.setProperty('--focus-ring', accent);
  root.style.setProperty('--accent-text', readableTextOn(accent));
}
