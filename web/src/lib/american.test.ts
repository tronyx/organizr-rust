import { describe, expect, it } from 'vitest';

import { american } from './american';

describe('american', () => {
  it('spells the British words of PHP the American way', () => {
    expect(american('Colour')).toBe('Color');
    expect(american('Pick the colours')).toBe('Pick the colors');
    expect(american('Donation Cancelled')).toBe('Donation Canceled');
    expect(american('The colour was cancelled.')).toBe('The color was canceled.');
  });

  it('changes whole words only', () => {
    expect(american('netdata1Colour')).toBe('netdata1Colour');
    expect(american('Colourful')).toBe('Colourful');
    expect(american('Colors & Themes')).toBe('Colors & Themes');
  });

  it('keeps a missing text missing', () => {
    expect(american(undefined)).toBeUndefined();
  });
});
