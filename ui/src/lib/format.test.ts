import { describe, expect, it } from 'vitest';
import { formatBytes } from './format';

describe('formatBytes', () => {
  it('[I18N-003] [CAP-006] shows size limits in KB or MB for the locale', () => {
    expect(formatBytes(256 * 1024, 'en')).toBe('256 kB');
    expect(formatBytes(1024 * 1024, 'en')).toBe('1 MB');
    expect(formatBytes(1536 * 1024, 'pt-BR')).toBe('1,5 MB');
  });
});
