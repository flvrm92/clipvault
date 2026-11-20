import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn<(...args: unknown[]) => Promise<undefined>>(async () => undefined);
const listen = vi.fn(async (_event: string, handler: (e: { payload: unknown }) => void) => {
  handler({ payload: 'payload' });
  return () => {};
});

vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (e: { payload: unknown }) => void) => listen(event, handler),
}));

const { imageUrl, tauriApi, thumbnailUrl } = await import('./client');

describe('tauriApi', () => {
  beforeEach(() => invoke.mockClear());

  it('maps every call to its Tauri command and arguments', async () => {
    const query = { filter: 'all', search: null, offset: 0, limit: 50 } as const;
    await tauriApi.listHistory(query);
    await tauriApi.getItemText(1);
    await tauriApi.pasteItem(2, 'trimmed');
    await tauriApi.copyItem(3);
    await tauriApi.togglePin(4);
    await tauriApi.deleteItem(5);
    await tauriApi.clearHistory('everything');
    await tauriApi.setCaptureState('paused');
    await tauriApi.getPopupState();
    await tauriApi.hidePopup();
    await tauriApi.pasteSnippet(6);
    await tauriApi.listSnippets();
    await tauriApi.getShortcutSettings();
    const draft = { id: null, name: 'n', body: 'b', hotkey: null, enabled: true };
    await tauriApi.saveSnippet(draft);
    await tauriApi.deleteSnippet(7);
    await tauriApi.changeOpenHotkey('Alt+V');
    await tauriApi.setHotkeySuspension('suspended');
    await tauriApi.setRestoreClipboard('off');
    await tauriApi.openSettings();
    const retention = { maxAgeDays: 7, maxItems: null };
    await tauriApi.getSettings();
    await tauriApi.setLanguage('ptBr');
    await tauriApi.setTheme('dark');
    await tauriApi.setUseAccentColor('on');
    await tauriApi.previewRetention(retention);
    await tauriApi.setRetention(retention);
    await tauriApi.setClearOnExit('on');
    await tauriApi.setSizeLimit('text', 1024);
    await tauriApi.addExcludedApp('keepass.exe');
    await tauriApi.removeExcludedApp('keepass.exe');
    await tauriApi.listRunningApps();
    await tauriApi.restoreDefaults('history');
    await tauriApi.getAbout();
    await tauriApi.openAboutLink('support');
    await tauriApi.openDataFolder();
    await tauriApi.getStartup();
    await tauriApi.setStartup('on');
    await tauriApi.getWelcome();
    await tauriApi.completeWelcome(true, false);

    expect(invoke.mock.calls).toEqual([
      ['list_history', { query }],
      ['get_item_text', { id: 1 }],
      ['paste_item', { id: 2, mode: 'trimmed' }],
      ['copy_item', { id: 3 }],
      ['toggle_pin', { id: 4 }],
      ['delete_item', { id: 5 }],
      ['clear_history', { scope: 'everything' }],
      ['set_capture_state', { state: 'paused' }],
      ['get_popup_state'],
      ['hide_popup'],
      ['paste_snippet', { id: 6 }],
      ['list_snippets'],
      ['get_shortcut_settings'],
      ['save_snippet', { draft }],
      ['delete_snippet', { id: 7 }],
      ['change_open_hotkey', { shortcut: 'Alt+V' }],
      ['set_hotkey_suspension', { suspension: 'suspended' }],
      ['set_restore_clipboard', { value: 'off' }],
      ['open_settings'],
      ['get_settings'],
      ['set_language', { language: 'ptBr' }],
      ['set_theme', { theme: 'dark' }],
      ['set_use_accent_color', { value: 'on' }],
      ['preview_retention', { retention }],
      ['set_retention', { retention }],
      ['set_clear_on_exit', { value: 'on' }],
      ['set_size_limit', { limit: 'text', bytes: 1024 }],
      ['add_excluded_app', { name: 'keepass.exe' }],
      ['remove_excluded_app', { name: 'keepass.exe' }],
      ['list_running_apps'],
      ['restore_defaults', { section: 'history' }],
      ['get_about'],
      ['open_about_link', { link: 'support' }],
      ['open_data_folder'],
      ['get_startup'],
      ['set_startup', { value: 'on' }],
      ['get_welcome'],
      ['complete_welcome', { startWithWindows: true, openSettings: false }],
    ]);
  });

  it('forwards event payloads to the handler', async () => {
    const handler = vi.fn();

    await tauriApi.on('history://changed', handler);

    expect(listen).toHaveBeenCalledWith('history://changed', expect.any(Function));
    expect(handler).toHaveBeenCalledWith('payload');
  });

  it('[SEC-003] builds image URLs from item ids only', () => {
    expect(thumbnailUrl(7)).toBe('http://clip.localhost/thumb/7');
    expect(imageUrl(7)).toBe('http://clip.localhost/image/7');
  });
});
