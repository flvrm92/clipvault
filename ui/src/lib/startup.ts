/** Start-with-Windows states the user cannot change from ClipVault (CFG-003). */
import type { StartupStateDto } from '../api/bindings/StartupStateDto';

/** Translation key explaining why the option is locked, or `null` when the user may change it. */
export function startupLock(state: StartupStateDto): string | null {
  switch (state) {
    case 'disabledByUser':
      return 'settings.general.startupLocked.byUser';
    case 'disabledByPolicy':
    case 'enabledByPolicy':
      return 'settings.general.startupLocked.byPolicy';
    default:
      return null;
  }
}

/** Whether ClipVault starts with Windows in `state`. */
export const startupEnabled = (state: StartupStateDto) =>
  state === 'enabled' || state === 'enabledByPolicy';
