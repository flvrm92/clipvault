import { describe, expect, it } from 'vitest';
import { t } from '../i18n';
import { errorMessage } from './errors';

describe('errorMessage', () => {
  it('[HK-003] names the snippet that already uses a shortcut', () => {
    expect(errorMessage({ code: 'hotkeyTakenBySnippet', detail: 'Assinatura', message: '' })).toBe(
      t('errors.hotkeyTakenBySnippet', { name: 'Assinatura' }),
    );
  });

  it('[HK-003] explains the violated shortcut rule', () => {
    expect(errorMessage({ code: 'invalidHotkey', detail: 'missingModifier', message: '' })).toBe(
      t('errors.hotkey.missingModifier'),
    );
  });

  it('[HK-004] reports shortcuts taken by other apps', () => {
    expect(errorMessage({ code: 'inUse', detail: null, message: '' })).toBe(t('errors.inUse'));
  });

  it('falls back to a generic message for anything unexpected', () => {
    expect(errorMessage(new Error('boom'))).toBe(t('errors.generic'));
    expect(errorMessage({ code: 'invalidSnippet', detail: 'weird', message: '' })).toBe(
      t('errors.generic'),
    );
  });
});
