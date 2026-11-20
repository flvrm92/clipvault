<script lang="ts">
  import type { ClipItemDto } from '../api/bindings/ClipItemDto';
  import { t } from '../i18n';
  import { visibleRange } from '../lib/virtual';
  import HistoryRow from './HistoryRow.svelte';

  /** Fixed row height: required by the virtualization (HIST-002). Keep in sync with CSS. */
  const ROW_HEIGHT = 68;
  const OVERSCAN = 4;

  let {
    items,
    selected,
    onactivate,
    onselect,
    ontogglepin,
    ondelete,
    onnearend,
    viewportRows = $bindable(6),
  }: {
    items: ClipItemDto[];
    selected: number;
    onactivate: (index: number) => void;
    onselect: (index: number) => void;
    ontogglepin: (index: number) => void;
    ondelete: (index: number) => void;
    onnearend: () => void;
    viewportRows?: number;
  } = $props();

  let viewport: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let height = $state(480);
  let now = $state(Date.now());

  const range = $derived(
    visibleRange({
      scrollTop,
      viewport: height,
      rowHeight: ROW_HEIGHT,
      count: items.length,
      overscan: OVERSCAN,
    }),
  );
  const activeId = $derived(items[selected] ? `clip-${items[selected]?.id}` : undefined);

  $effect(() => {
    viewportRows = Math.max(1, Math.floor(height / ROW_HEIGHT));
  });

  // Keep the selected row in view.
  $effect(() => {
    if (!viewport) return;
    const top = selected * ROW_HEIGHT;
    if (top < viewport.scrollTop) viewport.scrollTop = top;
    else if (top + ROW_HEIGHT > viewport.scrollTop + height)
      viewport.scrollTop = top + ROW_HEIGHT - height;
  });

  // Ask for the next page shortly before the end.
  $effect(() => {
    if (range.end >= items.length - OVERSCAN && items.length > 0) onnearend();
  });

  // Relative times refresh every 30 s.
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(timer);
  });
</script>

<div
  class="viewport"
  role="listbox"
  aria-label={t('popup.list.label')}
  aria-activedescendant={activeId}
  tabindex="-1"
  bind:this={viewport}
  bind:clientHeight={height}
  onscroll={() => (scrollTop = viewport?.scrollTop ?? 0)}
>
  <div class="spacer" style:height="{items.length * ROW_HEIGHT}px">
    <div class="window" style:transform="translateY({range.start * ROW_HEIGHT}px)">
      {#each items.slice(range.start, range.end) as item, offset (item.id)}
        {@const index = range.start + offset}
        <HistoryRow
          {item}
          {index}
          {now}
          selected={index === selected}
          onactivate={() => onactivate(index)}
          onselect={() => onselect(index)}
          ontogglepin={() => ontogglepin(index)}
          ondelete={() => ondelete(index)}
        />
      {/each}
    </div>
  </div>
</div>

<style>
  .viewport {
    --row-height: 68px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px;
    outline: none;
  }

  .spacer {
    position: relative;
  }

  .window {
    position: absolute;
    inset: 0 0 auto 0;
  }
</style>
