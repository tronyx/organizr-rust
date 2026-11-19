import { describe, expect, it } from 'vitest';

import { DEFAULT_REFRESH, refreshTime, SHORTEST_REFRESH } from './itemData.svelte';
import { fromNow } from './time';

describe('refreshTime', () => {
  it('is the setting, which the server sends as text', () => {
    expect(
      refreshTime({ homepageHealthChecksRefresh: '120000' }, 'homepageHealthChecksRefresh'),
    ).toBe(120000);
  });

  it('is never shorter than the shortest time', () => {
    expect(refreshTime({ a: '10' }, 'a')).toBe(SHORTEST_REFRESH);
  });

  it('is the default without a usable setting', () => {
    expect(refreshTime(undefined, 'a')).toBe(DEFAULT_REFRESH);
    expect(refreshTime({}, 'a')).toBe(DEFAULT_REFRESH);
    expect(refreshTime({ a: 'often' }, 'a')).toBe(DEFAULT_REFRESH);
    expect(refreshTime({ a: '0' }, 'a')).toBe(DEFAULT_REFRESH);
  });
});

describe('fromNow', () => {
  const now = Date.parse('2026-11-02T12:00:00Z');

  it('says a time in the largest unit that fits', () => {
    expect(fromNow('2026-11-02T09:00:00+00:00', now)).toBe('3 hours ago');
    expect(fromNow('2026-11-04T12:00:00+00:00', now)).toBe('in 2 days');
    expect(fromNow('2026-11-02T11:58:00+00:00', now)).toBe('2 minutes ago');
    expect(fromNow('2026-11-02T12:00:20+00:00', now)).toBe('in 20 seconds');
  });

  it('is nothing for no time', () => {
    expect(fromNow(null, now)).toBeNull();
    expect(fromNow('', now)).toBeNull();
    expect(fromNow('soon', now)).toBeNull();
  });
});
