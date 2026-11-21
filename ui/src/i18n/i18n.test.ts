import { describe, expect, it } from 'vitest';
import { flattenKeys, messages, resolveLocale, translate } from './index';

describe('resolveLocale', () => {
  it('[I18N-001] maps any Portuguese tag to pt-BR', () => {
    expect(resolveLocale('pt-BR')).toBe('pt-BR');
    expect(resolveLocale('pt')).toBe('pt-BR');
    expect(resolveLocale('PT-pt')).toBe('pt-BR');
  });

  it('[I18N-001] falls back to English for unsupported or missing tags', () => {
    expect(resolveLocale('en-US')).toBe('en');
    expect(resolveLocale('fr-FR')).toBe('en');
    expect(resolveLocale(undefined)).toBe('en');
    expect(resolveLocale('')).toBe('en');
  });
});

describe('translate', () => {
  const catalog = {
    en: { greeting: { hello: 'Hello' }, only: { en: 'English only' } },
    'pt-BR': { greeting: { hello: 'Olá' } },
  };

  it('resolves dotted keys in the requested locale', () => {
    expect(translate(catalog, 'pt-BR', 'greeting.hello')).toBe('Olá');
  });

  it('falls back to English, then to the key itself', () => {
    expect(translate(catalog, 'pt-BR', 'only.en')).toBe('English only');
    expect(translate(catalog, 'pt-BR', 'missing.key')).toBe('missing.key');
  });

  it('[I18N-003] replaces {placeholders} with parameters', () => {
    const withParams = { en: { found: 'No results for "{query}"' } };
    expect(translate(withParams, 'en', 'found', { query: 'café' })).toBe('No results for "café"');
    expect(translate(withParams, 'en', 'found')).toBe('No results for "{query}"');
  });

  it('does not return non-string nodes', () => {
    expect(translate(catalog, 'en', 'greeting')).toBe('greeting');
  });
});

describe('message catalogs', () => {
  it('[I18N-004] pt-BR and en declare exactly the same keys', () => {
    expect(flattenKeys(messages['pt-BR']).sort()).toEqual(flattenKeys(messages.en).sort());
  });

  it('[I18N-004] has no empty translations', () => {
    for (const locale of ['en', 'pt-BR'] as const) {
      for (const key of flattenKeys(messages[locale])) {
        expect(translate(messages, locale, key).trim(), `${locale}:${key}`).not.toBe('');
      }
    }
  });
});

describe('runtime locale', () => {
  it('[I18N-002] switching the locale changes translations without reloading', async () => {
    const { getLocale, setLocale, t } = await import('./index');

    setLocale('en');
    expect(t('tray.quit')).toBe('Quit');
    setLocale('pt-BR');
    expect(t('tray.quit')).toBe('Sair');
    expect(getLocale()).toBe('pt-BR');
    expect(document.documentElement.lang).toBe('pt-BR');
  });
});

describe('plurals', () => {
  it('[I18N-003] picks the plural form and formats the count for the locale', async () => {
    const { setLocale, tn } = await import('./index');

    setLocale('en');
    expect(tn('settings.history.removed', 1)).toBe('1 item removed.');
    expect(tn('settings.history.removed', 1500)).toBe('1,500 items removed.');
    setLocale('pt-BR');
    expect(tn('settings.history.removed', 1500)).toBe('1.500 itens removidos.');
  });
});
