/** Popup history state (HIST-001..009). UI-only state: the backend owns the data. */
import type { ClipVaultApi } from '../api/client';
import type { ClipItemDto } from '../api/bindings/ClipItemDto';
import type { KindFilterDto } from '../api/bindings/KindFilterDto';

export const FILTERS: KindFilterDto[] = ['all', 'text', 'images', 'pinned'];

export interface HistoryOptions {
  pageSize: number;
  debounceMs: number;
  undoMs: number;
}

const DEFAULTS: HistoryOptions = { pageSize: 50, debounceMs: 80, undoMs: 5000 };

export class HistoryStore {
  items = $state<ClipItemDto[]>([]);
  hasMore = $state(false);
  filter = $state<KindFilterDto>('all');
  search = $state('');
  selected = $state(0);
  pending = $state<{ item: ClipItemDto; index: number } | null>(null);

  readonly #api: ClipVaultApi;
  readonly #options: HistoryOptions;
  #request = 0;
  #searchTimer: ReturnType<typeof setTimeout> | undefined;
  #undoTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(api: ClipVaultApi, options: Partial<HistoryOptions> = {}) {
    this.#api = api;
    this.#options = { ...DEFAULTS, ...options };
  }

  get selectedItem(): ClipItemDto | undefined {
    return this.items[this.selected];
  }

  /** Back to the initial view: no search, all items, first item selected. */
  async reset(): Promise<void> {
    clearTimeout(this.#searchTimer);
    this.search = '';
    this.filter = 'all';
    this.selected = 0;
    await this.reload();
  }

  /** Reloads the loaded range (e.g. after `history://changed`). */
  async reload(): Promise<void> {
    const limit = Math.max(this.#options.pageSize, this.items.length);
    const request = ++this.#request;
    const page = await this.#api.listHistory({
      filter: this.filter,
      search: this.search.trim() || null,
      offset: 0,
      limit,
    });
    if (request !== this.#request) return; // a newer query superseded this one
    this.items = page.items.filter((item) => item.id !== this.pending?.item.id);
    this.hasMore = page.hasMore;
    this.selected = Math.min(this.selected, Math.max(0, this.items.length - 1));
  }

  async loadMore(): Promise<void> {
    if (!this.hasMore) return;
    const request = ++this.#request;
    const page = await this.#api.listHistory({
      filter: this.filter,
      search: this.search.trim() || null,
      offset: this.items.length,
      limit: this.#options.pageSize,
    });
    if (request !== this.#request) return;
    this.items = [...this.items, ...page.items];
    this.hasMore = page.hasMore;
  }

  setSearch(text: string): void {
    this.search = text;
    clearTimeout(this.#searchTimer);
    this.#searchTimer = setTimeout(() => {
      this.selected = 0;
      void this.reload();
    }, this.#options.debounceMs);
  }

  async cycleFilter(direction: 1 | -1): Promise<void> {
    const next = (FILTERS.indexOf(this.filter) + direction + FILTERS.length) % FILTERS.length;
    await this.setFilter(FILTERS[next] ?? 'all');
  }

  async setFilter(filter: KindFilterDto): Promise<void> {
    this.filter = filter;
    this.selected = 0;
    await this.reload();
  }

  move(by: number): void {
    this.select(this.selected + by);
  }

  page(direction: 1 | -1, rows: number): void {
    this.select(this.selected + direction * Math.max(1, rows));
  }

  edge(edge: 'first' | 'last'): void {
    this.select(edge === 'first' ? 0 : this.items.length - 1);
  }

  select(index: number): void {
    this.selected = Math.min(Math.max(0, index), Math.max(0, this.items.length - 1));
  }

  /** Hides the item now and deletes it after the undo delay (HIST-006). */
  remove(item: ClipItemDto): void {
    void this.flush();
    const index = this.items.findIndex((i) => i.id === item.id);
    if (index < 0) return;
    this.items = this.items.filter((i) => i.id !== item.id);
    this.select(this.selected);
    this.pending = { item, index };
    this.#undoTimer = setTimeout(() => void this.flush(), this.#options.undoMs);
  }

  undo(): void {
    if (!this.pending) return;
    clearTimeout(this.#undoTimer);
    const { item, index } = this.pending;
    this.items = [...this.items.slice(0, index), item, ...this.items.slice(index)];
    this.selected = index;
    this.pending = null;
  }

  /** Commits a pending deletion immediately (popup closing, another deletion). */
  async flush(): Promise<void> {
    const pending = this.pending;
    if (!pending) return;
    clearTimeout(this.#undoTimer);
    this.pending = null;
    await this.#api.deleteItem(pending.item.id);
  }
}
