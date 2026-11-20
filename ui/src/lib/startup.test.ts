import { describe, expect, it } from 'vitest';
import { startupEnabled, startupLock } from './startup';

describe('startup', () => {
  it('[CFG-003] only user-changeable states are unlocked', () => {
    expect(startupLock('enabled')).toBeNull();
    expect(startupLock('disabled')).toBeNull();
    expect(startupLock('disabledByUser')).toBe('settings.general.startupLocked.byUser');
    expect(startupLock('disabledByPolicy')).toBe('settings.general.startupLocked.byPolicy');
    expect(startupLock('enabledByPolicy')).toBe('settings.general.startupLocked.byPolicy');
  });

  it('[CFG-003] policy-enabled counts as enabled', () => {
    expect(startupEnabled('enabledByPolicy')).toBe(true);
    expect(startupEnabled('disabledByUser')).toBe(false);
  });
});
