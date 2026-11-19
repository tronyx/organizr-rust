import { describe, expect, it, vi } from 'vitest';

import { API_KEY_LENGTH, randomKey } from './random';

describe('randomKey', () => {
  it('has the length of an API key and only lower-case letters and digits', () => {
    for (let round = 0; round < 50; round += 1) {
      expect(randomKey()).toMatch(/^[a-z0-9]{20}$/);
    }
    expect(API_KEY_LENGTH).toBe(20);
    expect(randomKey(7)).toHaveLength(7);
  });

  it('differs from call to call', () => {
    const keys = new Set(Array.from({ length: 50 }, () => randomKey()));

    expect(keys.size).toBe(50);
  });

  it('throws away the bytes that would make some characters more likely', () => {
    // 252 is the first value that is thrown away: 252 = 7 * 36. The values 0 and 37 give
    // the characters "a" and "b".
    const bytes = [252, 255, 0, 37];
    vi.spyOn(crypto, 'getRandomValues').mockImplementation((array) => {
      if (array instanceof Uint8Array) {
        array.forEach((_, index) => {
          array[index] = bytes[index % bytes.length] ?? 0;
        });
      }
      return array;
    });

    expect(randomKey(4)).toBe('abab');
  });
});
