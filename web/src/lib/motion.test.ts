import { describe, expect, it, vi } from 'vitest';

import { motion, reducedMotion } from './motion';

function asks(reduce: boolean) {
  vi.stubGlobal('matchMedia', (query: string) => ({
    media: query,
    matches: query.includes('prefers-reduced-motion') && reduce,
  }));
}

describe('motion', () => {
  it('gives the duration when the person asked for no less motion', () => {
    asks(false);
    expect(reducedMotion()).toBe(false);
    expect(motion(150)).toBe(150);
  });

  it('gives 0 when reduced motion is asked', () => {
    asks(true);
    expect(reducedMotion()).toBe(true);
    expect(motion(150)).toBe(0);
  });

  it('gives the duration in a browser without media queries', () => {
    vi.stubGlobal('matchMedia', undefined);
    expect(motion(150)).toBe(150);
  });
});
