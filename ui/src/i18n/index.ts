import en from './en.json';
import ptBR from './pt-BR.json';

export type Locale = 'en' | 'pt-BR';
export interface MessageTree {
  [key: string]: string | MessageTree;
}
/** English is mandatory: it is the fallback for every other locale (I18N-001). */
export type Catalog = Partial<Record<Locale, MessageTree>> & { en: MessageTree };

/** The app's catalogs: every supported locale must be present. */
export const messages: Record<Locale, MessageTree> = { en, 'pt-BR': ptBR };

/** Maps a BCP 47 tag (e.g. `navigator.language`) to a supported locale (I18N-001). */
export function resolveLocale(tag: string | undefined): Locale {
  return tag?.toLowerCase().split('-')[0] === 'pt' ? 'pt-BR' : 'en';
}

function lookup(tree: MessageTree | undefined, key: string): string | undefined {
  let node: string | MessageTree | undefined = tree;
  for (const part of key.split('.')) {
    if (typeof node !== 'object') return undefined;
    node = node[part];
  }
  return typeof node === 'string' ? node : undefined;
}

/** Translates a dotted key, falling back to English and then to the key itself. */
export function translate(catalog: Catalog, locale: Locale, key: string): string {
  return lookup(catalog[locale], key) ?? lookup(catalog.en, key) ?? key;
}

/** Lists every leaf key of a message tree using dotted notation. */
export function flattenKeys(tree: MessageTree, prefix = ''): string[] {
  return Object.entries(tree).flatMap(([name, value]) => {
    const key = prefix ? `${prefix}.${name}` : name;
    return typeof value === 'string' ? [key] : flattenKeys(value, key);
  });
}

const currentLocale = resolveLocale(globalThis.navigator?.language);

/** Translates `key` in the current UI locale. */
export function t(key: string): string {
  return translate(messages, currentLocale, key);
}
