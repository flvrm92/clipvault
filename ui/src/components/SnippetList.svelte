<script lang="ts">
  import type { SnippetDto } from '../api/bindings/SnippetDto';
  import { t } from '../i18n';
  import { keycaps } from '../lib/shortcuts';

  /** Snippets in the popup (SNP-007): selecting one pastes it. */
  let {
    snippets,
    selected,
    onactivate,
    onselect,
  }: {
    snippets: SnippetDto[];
    selected: number;
    onactivate: (index: number) => void;
    onselect: (index: number) => void;
  } = $props();

  const activeId = $derived(snippets[selected] ? `snippet-${snippets[selected]?.id}` : undefined);
</script>

<div
  class="list"
  id="snippet-list"
  role="listbox"
  aria-label={t('popup.filter.snippets')}
  aria-activedescendant={activeId}
  tabindex="-1"
>
  {#each snippets as snippet, index (snippet.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events (keyboard handled by the popup) -->
    <div
      id="snippet-{snippet.id}"
      class="row"
      class:selected={index === selected}
      role="option"
      aria-selected={index === selected}
      tabindex="-1"
      onclick={() => onactivate(index)}
      onmousemove={index === selected ? undefined : () => onselect(index)}
    >
      <span class="top">
        <strong>{snippet.name}</strong>
        <span class="keys">
          {#each keycaps(snippet.hotkey) as cap (cap)}<kbd>{cap}</kbd>{/each}
        </span>
      </span>
      <span class="body">{snippet.body}</span>
    </div>
  {/each}
</div>

<style>
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px;
    outline: none;
  }

  .row {
    position: relative;
    display: grid;
    gap: 2px;
    padding: 8px 14px 8px 16px;
    border-radius: 6px;
    user-select: none;
  }

  .row:hover,
  .row.selected {
    background: var(--surface-hover);
  }

  .row.selected::before {
    content: '';
    position: absolute;
    left: 4px;
    top: 50%;
    width: 3px;
    height: 18px;
    border-radius: 2px;
    background: var(--accent);
    transform: translateY(-50%);
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
  }

  .keys {
    display: flex;
    gap: 3px;
  }

  kbd {
    padding: 0 5px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--surface-raised);
    font-family: inherit;
    font-size: 11px;
  }

  .body {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
