import { describe, expect, it } from 'vitest';
import { relativeTime } from './time';

const now = Date.UTC(2026, 10, 20, 12, 0, 0);
const ago = (ms: number) => now - ms;
const MIN = 60_000;

describe('relativeTime (HIST-001, I18N-003)', () => {
  it('shows "now" under a minute', () => {
    expect(relativeTime(ago(30_000), now, 'pt-BR')).toBe('agora');
    expect(relativeTime(ago(30_000), now, 'en')).toBe('now');
  });

  it('uses minutes, hours and days', () => {
    expect(relativeTime(ago(5 * MIN), now, 'pt-BR')).toBe('há 5 minutos');
    expect(relativeTime(ago(3 * 60 * MIN), now, 'en')).toBe('3 hours ago');
    expect(relativeTime(ago(24 * 60 * MIN), now, 'pt-BR')).toBe('ontem');
    expect(relativeTime(ago(3 * 24 * 60 * MIN), now, 'en')).toBe('3 days ago');
  });

  it('treats future timestamps (clock skew) as now', () => {
    expect(relativeTime(now + 5_000, now, 'en')).toBe('now');
  });
});
