<script lang="ts">
  import type { ClipItemDto } from '../api/bindings/ClipItemDto';
  import { thumbnailUrl } from '../api/client';
  import { getLocale, t } from '../i18n';
  import { relativeTime } from '../lib/time';
  import Icon from './Icon.svelte';

  let {
    item,
    selected,
    now,
    index,
    onactivate,
    onselect,
    ontogglepin,
    ondelete,
  }: {
    item: ClipItemDto;
    selected: boolean;
    now: number;
    index: number;
    onactivate: () => void;
    onselect: () => void;
    ontogglepin: () => void;
    ondelete: () => void;
  } = $props();

  const imageLabel = $derived(
    t('popup.item.image', { width: item.width ?? 0, height: item.height ?? 0 }),
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events (keyboard handled by the popup listbox) -->
<div
  id="clip-{item.id}"
  class="row"
  class:selected
  role="option"
  aria-selected={selected}
  aria-label={item.kind === 'image' ? imageLabel : item.preview}
  tabindex="-1"
  data-index={index}
  onclick={onactivate}
  onmousemove={selected ? undefined : onselect}
>
  {#if item.kind === 'image'}
    <img class="thumb" src={thumbnailUrl(item.id)} alt="" loading="lazy" draggable="false" />
    <span class="dimensions">{item.width} × {item.height}</span>
  {:else}
    <span class="preview">{item.preview}</span>
  {/if}

  <span class="meta">
    {#if item.pinned}
      <span class="pinned" title={t('popup.item.pinned')}><Icon name="pinFilled" size={14} /></span>
    {/if}
    <span class="time">{relativeTime(item.lastUsedAt, now, getLocale())}</span>
  </span>

  <!-- Pointer shortcuts only: Ctrl+P and Delete do the same from the keyboard (A11Y-001), and
       interactive content inside an option is not allowed (A11Y-003). -->
  <span class="actions" aria-hidden="true">
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <span
      class="action"
      title={item.pinned ? t('popup.item.unpin') : t('popup.item.pin')}
      onclick={(e) => {
        e.stopPropagation();
        ontogglepin();
      }}
    >
      <Icon name={item.pinned ? 'pinFilled' : 'pin'} />
    </span>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <span
      class="action"
      title={t('popup.item.delete')}
      onclick={(e) => {
        e.stopPropagation();
        ondelete();
      }}
    >
      <Icon name="delete" />
    </span>
  </span>
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    height: var(--row-height);
    padding: 7px 14px 7px 16px;
    border-radius: 6px;
    cursor: default;
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

  .preview {
    flex: 1;
    min-width: 0;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: pre-line;
    overflow-wrap: anywhere;
    font-size: 13px;
    line-height: 18px;
  }

  .thumb {
    width: 96px;
    height: 56px;
    object-fit: cover;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .dimensions {
    flex: 1;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .meta {
    align-self: flex-start;
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: 11px;
    white-space: nowrap;
  }

  .pinned {
    display: inline-flex;
    color: var(--accent);
  }

  .actions {
    position: absolute;
    right: 10px;
    bottom: 8px;
    display: none;
    gap: 2px;
  }

  .row:hover .actions,
  .row.selected .actions {
    display: flex;
  }

  .action {
    cursor: pointer;
    display: inline-flex;
    padding: 4px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
  }

  .action:hover {
    background: var(--surface-selected);
    color: var(--text-primary);
  }
</style>
