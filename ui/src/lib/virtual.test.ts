import { describe, expect, it } from 'vitest';
import { visibleRange } from './virtual';

describe('visibleRange (HIST-002)', () => {
  it('renders only the rows in view plus overscan', () => {
    expect(
      visibleRange({ scrollTop: 0, viewport: 300, rowHeight: 76, count: 5000, overscan: 3 }),
    ).toEqual({
      start: 0,
      end: 7,
    });
    expect(
      visibleRange({ scrollTop: 760, viewport: 300, rowHeight: 76, count: 5000, overscan: 3 }),
    ).toEqual({ start: 7, end: 17 });
  });

  it('never exceeds the list bounds', () => {
    expect(
      visibleRange({ scrollTop: 10_000, viewport: 300, rowHeight: 76, count: 20, overscan: 3 }),
    ).toEqual({
      start: 13,
      end: 20,
    });
    expect(
      visibleRange({ scrollTop: 0, viewport: 300, rowHeight: 76, count: 0, overscan: 3 }),
    ).toEqual({
      start: 0,
      end: 0,
    });
  });
});
