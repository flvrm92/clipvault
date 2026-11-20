import { vi } from 'vitest';
import type { AppEventName, ClipVaultApi } from '../api/client';
import type { ClipItemDto } from '../api/bindings/ClipItemDto';
import type { HistoryQueryDto } from '../api/bindings/HistoryQueryDto';
import type { PopupStateDto } from '../api/bindings/PopupStateDto';

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

/** An in-memory backend that filters and pages `items` like the real repository. */
export function fakeApi(items: ClipItemDto[], state: Partial<PopupStateDto> = {}) {
  let data = [...items];
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
    on: vi.fn(async (event: AppEventName, handler: (payload: unknown) => void) => {
      handlers.set(event, [...(handlers.get(event) ?? []), handler]);
      return () => {};
    }),
  } satisfies ClipVaultApi;

  return {
    api,
    emit: (event: AppEventName, payload?: unknown) =>
      handlers.get(event)?.forEach((handler) => handler(payload)),
  };
}
