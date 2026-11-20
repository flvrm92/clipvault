/** Relative timestamps for history rows (HIST-001, I18N-003). */

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const NOW: Record<string, string> = { 'pt-BR': 'agora', en: 'now' };

export function relativeTime(timestamp: number, now: number, locale: string): string {
  const elapsed = Math.max(0, now - timestamp);
  if (elapsed < MINUTE) return NOW[locale] ?? 'now';
  const format = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  if (elapsed < HOUR) return format.format(-Math.floor(elapsed / MINUTE), 'minute');
  if (elapsed < DAY) return format.format(-Math.floor(elapsed / HOUR), 'hour');
  if (elapsed < 30 * DAY) return format.format(-Math.floor(elapsed / DAY), 'day');
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(timestamp);
}
