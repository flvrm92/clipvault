/** Translated messages for backend errors (specs/03 §6). */
import type { CommandError } from '../api/bindings/CommandError';
import { messages, t } from '../i18n';

function isCommandError(error: unknown): error is CommandError {
  return typeof error === 'object' && error !== null && 'code' in error;
}

function known(key: string): boolean {
  let node: unknown = messages.en;
  for (const part of key.split('.')) {
    if (typeof node !== 'object' || node === null) return false;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === 'string';
}

export function errorMessage(error: unknown): string {
  if (!isCommandError(error)) return t('errors.generic');
  switch (error.code) {
    case 'hotkeyTakenBySnippet':
      return t('errors.hotkeyTakenBySnippet', { name: error.detail ?? '' });
    case 'invalidHotkey':
    case 'invalidSnippet': {
      const group = error.code === 'invalidHotkey' ? 'hotkey' : 'snippet';
      const key = `errors.${group}.${error.detail ?? ''}`;
      return known(key) ? t(key) : t('errors.generic');
    }
    default: {
      const key = `errors.${error.code}`;
      return known(key) ? t(key) : t('errors.generic');
    }
  }
}
