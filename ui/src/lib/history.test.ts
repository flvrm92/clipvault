import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HistoryStore } from './history.svelte';
import { fakeApi, imageItem, textItem } from '../test/fakeApi';

const many = (n: number) => Array.from({ length: n }, (_, i) => textItem(i + 1, `item ${i + 1}`));

describe('HistoryStore', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('[HIST-001] loads the first page and selects the first item', async () => {
    const { api } = fakeApi(many(3));
    const store = new HistoryStore(api);

    await store.reset();

    expect(store.items.map((i) => i.id)).toEqual([1, 2, 3]);
    expect(store.selectedItem?.id).toBe(1);
  });

  it('[HIST-002] loads further pages on demand', async () => {
    const { api } = fakeApi(many(120));
    const store = new HistoryStore(api, { pageSize: 50 });
    await store.reset();

    expect(store.hasMore).toBe(true);
    await store.loadMore();

    expect(store.items).toHaveLength(100);
    expect(api.listHistory).toHaveBeenLastCalledWith(
      expect.objectContaining({ offset: 50, limit: 50 }),
    );
  });

  it('[HIST-003] searches after a short debounce', async () => {
    const { api } = fakeApi([textItem(1, 'Café'), textItem(2, 'Chá')]);
    const store = new HistoryStore(api, { debounceMs: 80 });
    await store.reset();

    store.setSearch('c');
    store.setSearch('ca');
    store.setSearch('caf');
    await vi.advanceTimersByTimeAsync(80);

    expect(api.listHistory).toHaveBeenCalledTimes(2); // reset + one debounced search
    expect(api.listHistory).toHaveBeenLastCalledWith(expect.objectContaining({ search: 'caf' }));
    expect(store.items.map((i) => i.id)).toEqual([1]);
  });

  it('[HIST-004] changing the filter reloads and resets the selection', async () => {
    const { api } = fakeApi([textItem(1, 'a'), imageItem(2)]);
    const store = new HistoryStore(api);
    await store.reset();
    store.move(1);

    await store.cycleFilter(1); // all → text
    expect(store.filter).toBe('text');
    await store.cycleFilter(1); // text → images
    expect(store.items.map((i) => i.id)).toEqual([2]);
    expect(store.selected).toBe(0);
    await store.cycleFilter(-1);
    await store.cycleFilter(-1);
    await store.cycleFilter(-1); // wraps around to pinned
    expect(store.filter).toBe('pinned');
  });

  it('[A11Y-001] the selection stays within bounds', async () => {
    const { api } = fakeApi(many(5));
    const store = new HistoryStore(api);
    await store.reset();

    store.move(-1);
    expect(store.selected).toBe(0);
    store.page(1, 3);
    expect(store.selected).toBe(3);
    store.page(1, 3);
    expect(store.selected).toBe(4);
    store.edge('first');
    expect(store.selected).toBe(0);
    store.edge('last');
    expect(store.selected).toBe(4);
  });

  it('[HIST-006] a deletion can be undone within the delay, otherwise it is committed', async () => {
    const { api } = fakeApi(many(3));
    const store = new HistoryStore(api, { undoMs: 5000 });
    await store.reset();

    store.remove(store.items[0]!);
    expect(store.items.map((i) => i.id)).toEqual([2, 3]);
    store.undo();
    expect(store.items.map((i) => i.id)).toEqual([1, 2, 3]);

    store.remove(store.items[1]!);
    await vi.advanceTimersByTimeAsync(5000);
    expect(api.deleteItem).toHaveBeenCalledWith(2);
    expect(store.pending).toBeNull();
  });

  it('[HIST-006] a pending deletion is committed immediately when the popup closes', async () => {
    const { api } = fakeApi(many(2));
    const store = new HistoryStore(api);
    await store.reset();

    store.remove(store.items[0]!);
    await store.flush();

    expect(api.deleteItem).toHaveBeenCalledWith(1);
  });

  it('[HIST-009] reloading keeps a pending deletion hidden', async () => {
    const { api } = fakeApi(many(3));
    const store = new HistoryStore(api);
    await store.reset();

    store.remove(store.items[0]!);
    await store.reload();

    expect(store.items.map((i) => i.id)).toEqual([2, 3]);
  });
});
