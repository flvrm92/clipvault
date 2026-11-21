import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import BrandMark from './BrandMark.svelte';

describe('BrandMark', () => {
  it('uses the flat symbol up to 32 px and the detailed one above (specs/07 §1)', () => {
    const small = render(BrandMark, { size: 20 }).container;
    const large = render(BrandMark, { size: 56 }).container;

    expect(small.querySelector('linearGradient')).toBeNull();
    expect(large.querySelector('linearGradient')).not.toBeNull();
    expect(small.querySelector('.mark')).toHaveStyle({ width: '20px' });
  });
});
