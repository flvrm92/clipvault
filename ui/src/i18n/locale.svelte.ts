/** The active interface locale, as reactive state so translated text updates in place
 * when the backend reports a language change (I18N-002). */
export const localeState = $state<{ tag: string }>({ tag: globalThis.navigator?.language ?? 'en' });
