/** Window of rows to render in a virtualized list with fixed row height (HIST-002). */

export interface RangeInput {
  scrollTop: number;
  viewport: number;
  rowHeight: number;
  count: number;
  overscan: number;
}

/** Half-open range `[start, end)` of row indexes to render. */
export function visibleRange({ scrollTop, viewport, rowHeight, count, overscan }: RangeInput) {
  const inView = Math.ceil(viewport / rowHeight);
  const first = Math.min(Math.floor(scrollTop / rowHeight), Math.max(0, count - inView));
  return {
    start: Math.max(0, first - overscan),
    end: Math.min(count, first + inView + overscan),
  };
}
