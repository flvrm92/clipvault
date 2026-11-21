<script lang="ts">
  import { onMount } from 'svelte';
  import { imageUrl, type ClipVaultApi } from '../api/client';
  import type { ClipItemDto } from '../api/bindings/ClipItemDto';
  import type { KindFilterDto } from '../api/bindings/KindFilterDto';
  import type { PopupStateDto } from '../api/bindings/PopupStateDto';
  import type { SnippetDto } from '../api/bindings/SnippetDto';
  import { setLocale, t } from '../i18n';
  import { applyAppearance } from '../lib/appearance';
  import { FILTERS, HistoryStore } from '../lib/history.svelte';
  import { resolveKey, type KeyAction } from '../lib/keyboard';
  import BrandMark from '../lib/BrandMark.svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import HistoryList from './HistoryList.svelte';
  import Icon from './Icon.svelte';
  import SnippetList from './SnippetList.svelte';

  let { api }: { api: ClipVaultApi } = $props();

  // svelte-ignore state_referenced_locally (the api is fixed for the component's lifetime)
  const store = new HistoryStore(api);
  let popupState = $state<PopupStateDto | null>(null);
  let search: HTMLInputElement | undefined = $state();
  let viewportRows = $state(6);
  let menuOpen = $state(false);
  let confirmingClear = $state(false);
  let preview = $state<{ item: ClipItemDto; text: string } | null>(null);
  let failed = $state(false);

  type Tab = KindFilterDto | 'snippets';
  const TABS: Tab[] = [...FILTERS, 'snippets'];
  let tab = $state<Tab>('all');
  let snippets = $state<SnippetDto[]>([]);
  let snippetSelected = $state(0);

  const query = $derived(store.search.trim());
  const fold = (text: string) => text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
  const visibleSnippets = $derived(
    query ? snippets.filter((s) => fold(`${s.name} ${s.body}`).includes(fold(query))) : snippets,
  );

  async function selectTab(next: Tab) {
    tab = next;
    snippetSelected = 0;
    if (next === 'snippets') snippets = await api.listSnippets();
    else await store.setFilter(next);
  }

  function moveSnippet(index: number) {
    snippetSelected = Math.min(Math.max(0, index), Math.max(0, visibleSnippets.length - 1));
  }

  const pasteSnippet = (snippet: SnippetDto | undefined) =>
    snippet && guard(() => api.pasteSnippet(snippet.id));

  /** Keyboard actions while the snippet tab is shown (SNP-007). */
  function runOnSnippets(action: KeyAction) {
    switch (action.type) {
      case 'move':
        return moveSnippet(snippetSelected + action.by);
      case 'page':
        return moveSnippet(snippetSelected + action.direction * viewportRows);
      case 'edge':
        return moveSnippet(action.edge === 'first' ? 0 : visibleSnippets.length - 1);
      case 'paste':
        return pasteSnippet(visibleSnippets[snippetSelected]);
      case 'pasteNth':
        return pasteSnippet(visibleSnippets[action.index]);
      default:
        return undefined;
    }
  }

  async function guard(action: () => Promise<unknown>) {
    try {
      failed = false;
      await action();
    } catch (error) {
      console.error(error);
      failed = true;
    }
  }

  async function refreshState() {
    popupState = await api.getPopupState();
    applyAppearance(popupState.appearance);
    setLocale(popupState.locale);
  }

  async function opened() {
    menuOpen = false;
    confirmingClear = false;
    preview = null;
    tab = 'all';
    await Promise.all([refreshState(), store.reset()]);
    search?.focus();
  }

  const paste = (item: ClipItemDto | undefined, mode: 'asIs' | 'trimmed' = 'asIs') =>
    item && guard(() => api.pasteItem(item.id, mode));

  const togglePin = (item: ClipItemDto | undefined) =>
    item &&
    guard(async () => {
      await api.togglePin(item.id);
      await store.reload();
    });

  async function togglePreview(item: ClipItemDto | undefined) {
    if (preview || !item) {
      preview = null;
      return;
    }
    const text = item.kind === 'text' ? await api.getItemText(item.id) : '';
    preview = { item, text };
  }

  function run(action: KeyAction) {
    const item = store.selectedItem;
    if (action.type === 'filter') {
      const next = (TABS.indexOf(tab) + action.direction + TABS.length) % TABS.length;
      return selectTab(TABS[next] ?? 'all');
    }
    if (action.type === 'settings') return api.openSettings();
    if (tab === 'snippets' && !['clearSearch', 'close'].includes(action.type)) {
      return runOnSnippets(action);
    }
    switch (action.type) {
      case 'move':
        return store.move(action.by);
      case 'page':
        return store.page(action.direction, viewportRows);
      case 'edge':
        return store.edge(action.edge);
      case 'paste':
        return paste(item, action.mode);
      case 'pasteNth':
        return paste(store.items[action.index]);
      case 'copy':
        return item && guard(() => api.copyItem(item.id));
      case 'togglePin':
        return togglePin(item);
      case 'delete':
        return item && store.remove(item);
      case 'preview':
        return togglePreview(item);
      case 'clearSearch':
        return store.setSearch('');
      case 'close':
        return preview ? (preview = null) : api.hidePopup();
      default:
        return undefined;
    }
  }

  function onkeydown(event: KeyboardEvent) {
    const input = search;
    const action = resolveKey(
      { key: event.key, shift: event.shiftKey, ctrl: event.ctrlKey, alt: event.altKey },
      {
        searchEmpty: store.search.length === 0,
        inputHasSelection: !!input && input.selectionStart !== input.selectionEnd,
        caretAtEnd: !!input && input.selectionStart === input.value.length,
      },
    );
    if (!action) return;
    event.preventDefault();
    void run(action);
  }

  async function setCapture(paused: boolean) {
    menuOpen = false;
    await guard(async () => {
      await api.setCaptureState(paused ? 'paused' : 'active');
      await refreshState();
    });
  }

  async function clearHistory() {
    confirmingClear = false;
    menuOpen = false;
    await guard(async () => {
      await api.clearHistory('unpinnedOnly');
      await store.reload();
    });
  }

  onMount(() => {
    const subscriptions = [
      api.on('popup://shown', () => void opened()),
      api.on('popup://hidden', () => void store.flush()),
      api.on('history://changed', () => void store.reload()),
      api.on('capture://changed', () => void refreshState()),
      api.on('appearance://changed', () => void refreshState()),
      api.on('settings://changed', () => void refreshState()),
    ];
    void opened();
    return () => subscriptions.forEach((s) => void s.then((unlisten) => unlisten()));
  });
</script>

<svelte:window {onkeydown} />

<div class="popup">
  <header data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>
      <BrandMark size={18} />
      <span data-tauri-drag-region>{t('app.name')}</span>
    </span>
    <span class="menu-anchor">
      <button
        type="button"
        class="icon-button"
        title={t('popup.menu.label')}
        aria-label={t('popup.menu.label')}
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <Icon name="more" />
      </button>
      {#if menuOpen}
        <div class="menu" role="menu">
          <button
            type="button"
            role="menuitem"
            onclick={() => setCapture(!popupState?.capturePaused)}
          >
            {popupState?.capturePaused ? t('popup.menu.resume') : t('popup.menu.pause')}
          </button>
          <button type="button" role="menuitem" onclick={() => (confirmingClear = true)}>
            {t('popup.menu.clear')}
          </button>
          <button type="button" role="menuitem" onclick={() => api.openSettings()}>
            {t('popup.menu.settings')}
          </button>
        </div>
      {/if}
    </span>
  </header>

  <label class="search">
    <Icon name="search" />
    <input
      bind:this={search}
      type="text"
      value={store.search}
      oninput={(e) => store.setSearch(e.currentTarget.value)}
      placeholder={t('popup.search.placeholder')}
      aria-label={t('popup.search.label')}
      spellcheck="false"
      autocomplete="off"
    />
  </label>

  <div class="filters" role="tablist" aria-label={t('popup.filter.label')}>
    {#each TABS as option (option)}
      <button
        type="button"
        role="tab"
        tabindex="-1"
        aria-selected={tab === option}
        onclick={() => selectTab(option)}
      >
        {t(`popup.filter.${option}`)}
      </button>
    {/each}
  </div>

  {#if popupState?.capturePaused}
    <div class="banner" role="status">
      <Icon name="pause" size={14} />
      <span>{t('popup.paused.message')}</span>
      <button type="button" onclick={() => setCapture(false)}>{t('popup.paused.resume')}</button>
    </div>
  {/if}

  {#if confirmingClear}
    <div class="confirm" role="alertdialog" aria-label={t('popup.clear.confirm')}>
      <strong>{t('popup.clear.confirm')}</strong>
      <span>{t('popup.clear.keepPinned')}</span>
      <div>
        <button type="button" onclick={() => (confirmingClear = false)}>
          {t('popup.clear.cancel')}
        </button>
        <button type="button" class="danger" onclick={clearHistory}>
          {t('popup.clear.confirmAction')}
        </button>
      </div>
    </div>
  {/if}

  {#if tab === 'snippets'}
    {#if visibleSnippets.length > 0}
      <SnippetList
        snippets={visibleSnippets}
        selected={snippetSelected}
        onactivate={(i) => pasteSnippet(visibleSnippets[i])}
        onselect={(i) => (snippetSelected = i)}
      />
    {:else if query}
      <p class="empty" role="status">{t('popup.snippets.noResults', { query })}</p>
    {:else}
      <p class="empty" role="status">{t('popup.snippets.empty')}</p>
    {/if}
  {:else if store.items.length > 0}
    <HistoryList
      items={store.items}
      selected={store.selected}
      bind:viewportRows
      onactivate={(i) => paste(store.items[i])}
      onselect={(i) => store.select(i)}
      ontogglepin={(i) => togglePin(store.items[i])}
      ondelete={(i) => {
        const item = store.items[i];
        if (item) store.remove(item);
      }}
      onnearend={() => void store.loadMore()}
    />
  {:else if query}
    <p class="empty" role="status">{t('popup.noResults', { query })}</p>
  {:else if store.filter !== 'all'}
    <p class="empty" role="status">{t('popup.noItemsInFilter')}</p>
  {:else}
    <div class="empty" role="status">
      <EmptyState />
      <strong>{t('popup.empty.title')}</strong>
      <span>{t('popup.empty.body')}</span>
      {#if popupState}
        <kbd>{t('popup.empty.shortcut', { shortcut: popupState.openShortcut })}</kbd>
      {/if}
    </div>
  {/if}

  {#if store.pending}
    <div class="toast" role="status">
      <span>{t('popup.toast.deleted')}</span>
      <button type="button" onclick={() => store.undo()}>{t('popup.toast.undo')}</button>
    </div>
  {:else if failed}
    <div class="toast error" role="alert">{t('popup.toast.error')}</div>
  {/if}

  {#if preview}
    <div class="preview" role="dialog" aria-label={preview.item.preview}>
      <button
        type="button"
        class="icon-button close"
        aria-label={t('popup.preview.close')}
        onclick={() => (preview = null)}
      >
        <Icon name="close" />
      </button>
      {#if preview.item.kind === 'image'}
        <img src={imageUrl(preview.item.id)} alt="" />
      {:else}
        <pre>{preview.text}</pre>
      {/if}
    </div>
  {/if}

  <footer>
    <span><kbd>↵</kbd> {t('popup.hints.paste')}</span>
    <span><kbd>⇧↵</kbd> {t('popup.hints.plain')}</span>
    <span><kbd>Ctrl+P</kbd> {t('popup.hints.pin')}</span>
    <span><kbd>Del</kbd> {t('popup.hints.delete')}</span>
  </footer>
</div>

<style>
  .popup {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--popup-background);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 8px 4px 14px;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 13px;
  }

  .icon-button {
    display: inline-flex;
    padding: 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
  }

  .icon-button:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .menu-anchor {
    position: relative;
  }

  .menu {
    position: absolute;
    right: 0;
    top: 32px;
    z-index: 5;
    display: grid;
    min-width: 180px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  .menu button {
    padding: 8px 10px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-primary);
    text-align: left;
  }

  .menu button:hover {
    background: var(--surface-hover);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 12px 8px;
    padding: 0 10px;
    height: 34px;
    border: 1px solid var(--border-strong);
    border-bottom: 2px solid var(--accent);
    border-radius: 6px;
    background: var(--surface-raised);
    color: var(--text-secondary);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
  }

  .filters {
    display: flex;
    gap: 4px;
    padding: 0 12px 8px;
  }

  .filters button {
    padding: 4px 10px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .filters button:hover {
    background: var(--surface-hover);
  }

  .filters button[aria-selected='true'] {
    border-color: var(--border);
    background: var(--surface-selected);
    color: var(--text-primary);
    font-weight: 600;
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 12px 8px;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--warning-surface);
    color: var(--warning);
    font-size: 12px;
  }

  .banner button,
  .toast button {
    margin-left: auto;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-weight: 600;
  }

  .confirm {
    display: grid;
    gap: 6px;
    margin: 0 12px 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
    font-size: 12px;
  }

  .confirm div {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .confirm button {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface);
    color: var(--text-primary);
  }

  .confirm .danger {
    border-color: var(--danger);
    background: var(--danger);
    color: var(--accent-text);
  }

  .empty {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 6px;
    margin: 0;
    padding: 24px;
    color: var(--text-secondary);
    text-align: center;
  }

  .empty strong {
    color: var(--text-primary);
    font-size: 15px;
  }

  kbd {
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--surface-raised);
    font-family: inherit;
    font-size: 11px;
  }

  .toast {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 44px;
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-radius: var(--radius);
    background: var(--inverse-surface);
    color: var(--inverse-text);
    font-size: 12px;
  }

  .toast button {
    color: var(--inverse-accent);
  }

  .toast.error {
    background: var(--danger);
    color: var(--accent-text);
  }

  .preview {
    position: absolute;
    inset: 44px 8px 40px;
    z-index: 4;
    padding: 12px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  .preview pre {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .preview img {
    max-width: 100%;
  }

  .preview .close {
    position: sticky;
    top: 0;
    float: right;
  }

  footer {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
  }
</style>
