/** Locale-aware number formats (I18N-003). */

const MB = 1024 * 1024;

/** A size limit in kB or MB (binary multiples, as offered by CAP-006). */
export function formatBytes(bytes: number, locale: string): string {
  const [value, unit] = bytes >= MB ? [bytes / MB, 'megabyte'] : [bytes / 1024, 'kilobyte'];
  return new Intl.NumberFormat(locale, {
    style: 'unit',
    unit,
    maximumFractionDigits: 1,
  }).format(value);
}
