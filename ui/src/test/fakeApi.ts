import { vi } from 'vitest';
import type { AppEventName, ClipVaultApi } from '../api/client';
import type { ClipItemDto } from '../api/bindings/ClipItemDto';
import type { HistoryQueryDto } from '../api/bindings/HistoryQueryDto';
import type { PopupStateDto } from '../api/bindings/PopupStateDto';
import type { CommandError } from '../api/bindings/CommandError';
import type { SnippetDraftDto } from '../api/bindings/SnippetDraftDto';
import type { SnippetDto } from '../api/bindings/SnippetDto';

export function textItem(
  id: number,
  preview: string,
  extra: Partial<ClipItemDto> = {},
): ClipItemDto {
  return {
    id,
    kind: 'text',
    preview,
    pinned: false,
    lastUsedAt: Date.now() - id * 60_000,
    sourceApp: null,
    width: null,
    height: null,
    ...extra,
  };
}

export function imageItem(id: number): ClipItemDto {
  return { ...textItem(id, ''), kind: 'image', width: 1920, height: 1080 };
}

export function snippet(id: number, name: string, extra: Partial<SnippetDto> = {}): SnippetDto {
  return { id, name, body: `corpo de ${name}`, hotkey: null, enabled: true, ...extra };
}

const fail = (error: CommandError) => Promise.reject(error);

/** An in-memory backend that filters and pages `items` like the real repository. */
export function fakeApi(
  items: ClipItemDto[],
  state: Partial<PopupStateDto> = {},
  initialSnippets: SnippetDto[] = [],
) {
  let data = [...items];
  let snippets = [...initialSnippets];
  let openShortcut = state.openShortcut ?? 'Ctrl+Win+V';
  let restoreClipboard = true;
  let nextSnippetId = 100;
  const blocked = new Set<string>();
  const handlers = new Map<AppEventName, ((payload: unknown) => void)[]>();
  const popupState: PopupStateDto = {
    openShortcut: 'Ctrl+Win+V',
    capturePaused: false,
    appearance: { theme: 'light', backdrop: 'solid' },
    ...state,
  };
  const matches = (item: ClipItemDto, q: HistoryQueryDto) =>
    (q.filter === 'all' ||
      (q.filter === 'text' && item.kind === 'text') ||
      (q.filter === 'images' && item.kind === 'image') ||
      (q.filter === 'pinned' && item.pinned)) &&
    (!q.search || item.preview.toLowerCase().includes(q.search.toLowerCase()));

  const api = {
    listHistory: vi.fn(async (q: HistoryQueryDto) => {
      const found = data.filter((i) => matches(i, q));
      return {
        items: found.slice(q.offset, q.offset + q.limit),
        hasMore: found.length > q.offset + q.limit,
      };
    }),
    getItemText: vi.fn(async (id: number) => data.find((i) => i.id === id)?.preview ?? ''),
    pasteItem: vi.fn(async () => 'pasted' as const),
    copyItem: vi.fn(async () => {}),
    togglePin: vi.fn(async (id: number) => {
      data = data.map((i) => (i.id === id ? { ...i, pinned: !i.pinned } : i));
      return data.find((i) => i.id === id)?.pinned ?? false;
    }),
    deleteItem: vi.fn(async (id: number) => {
      data = data.filter((i) => i.id !== id);
    }),
    clearHistory: vi.fn(async () => {
      const before = data.length;
      data = data.filter((i) => i.pinned);
      return before - data.length;
    }),
    setCaptureState: vi.fn(async (s: 'active' | 'paused') => {
      popupState.capturePaused = s === 'paused';
    }),
    getPopupState: vi.fn(async () => ({ ...popupState })),
    hidePopup: vi.fn(async () => {}),
    pasteSnippet: vi.fn(async () => 'pasted' as const),
    listSnippets: vi.fn(async () => [...snippets]),
    getShortcutSettings: vi.fn(async () => ({
      openShortcut,
      restoreClipboard,
      snippets: [...snippets],
    })),
    saveSnippet: vi.fn(async (draft: SnippetDraftDto) => {
      if (!draft.name.trim())
        return fail({ code: 'invalidSnippet', detail: 'nameBlank', message: '' });
      const owner = snippets.find(
        (s) => s.hotkey && s.hotkey === draft.hotkey && s.id !== draft.id,
      );
      if (owner) return fail({ code: 'hotkeyTakenBySnippet', detail: owner.name, message: '' });
      if (draft.hotkey && blocked.has(draft.hotkey))
        return fail({ code: 'inUse', detail: null, message: '' });
      const saved: SnippetDto = {
        id: draft.id ?? nextSnippetId++,
        name: draft.name.trim(),
        body: draft.body,
        hotkey: draft.hotkey,
        enabled: draft.enabled,
      };
      snippets =
        draft.id === null
          ? [...snippets, saved]
          : snippets.map((s) => (s.id === draft.id ? saved : s));
      return saved;
    }),
    deleteSnippet: vi.fn(async (id: number) => {
      snippets = snippets.filter((s) => s.id !== id);
    }),
    changeOpenHotkey: vi.fn(async (shortcut: string) => {
      if (blocked.has(shortcut)) return fail({ code: 'inUse', detail: null, message: '' });
      openShortcut = shortcut;
      return shortcut;
    }),
    setHotkeySuspension: vi.fn(async () => {}),
    setRestoreClipboard: vi.fn(async (value: 'on' | 'off') => {
      restoreClipboard = value === 'on';
    }),
    openSettings: vi.fn(async () => {}),
    on: vi.fn(async (event: AppEventName, handler: (payload: unknown) => void) => {
      handlers.set(event, [...(handlers.get(event) ?? []), handler]);
      return () => {};
    }),
  } satisfies ClipVaultApi;

  return {
    api,
    /** Simulates another application holding `shortcut` (HK-004). */
    block: (shortcut: string) => blocked.add(shortcut),
    emit: (event: AppEventName, payload?: unknown) =>
      handlers.get(event)?.forEach((handler) => handler(payload)),
  };
}
