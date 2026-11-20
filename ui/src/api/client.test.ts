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
