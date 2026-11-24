import { afterEach, describe, expect, it } from 'vitest';

import { DEFAULT_REFRESH, refreshTime, SHORTEST_REFRESH } from './itemData.svelte';
import { durationText, fromNow, localDateTime, timeSpanSeconds, timeSpanText } from './time';

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

describe('durationText', () => {
  it('says a number of seconds in its two largest units', () => {
    expect(durationText(45)).toBe('45 s');
    expect(durationText(720)).toBe('12 min');
    expect(durationText(3720)).toBe('1 h 2 min');
    expect(durationText(183600)).toBe('2 d 3 h');
    expect(durationText(-1)).toBe('');
    expect(durationText(Number.NaN)).toBe('');
  });
});

describe('timeSpanText', () => {
  it('says a time span of the *arr applications as a duration', () => {
    expect(timeSpanText('01:51:22.3339332')).toBe('1 h 51 min');
    expect(timeSpanText('2.03:04:05')).toBe('2 d 3 h');
    expect(timeSpanText('00:00:45')).toBe('45 s');
  });

  it('is empty for an empty or unknown text', () => {
    expect(timeSpanText('')).toBe('');
    expect(timeSpanText(null)).toBe('');
    expect(timeSpanText(undefined)).toBe('');
    expect(timeSpanText('junk')).toBe('');
    expect(timeSpanText('00:61:00')).toBe('');
  });

  it('is empty when no time is left', () => {
    expect(timeSpanText('00:00:00')).toBe('');
    expect(timeSpanText('00:00:00.0000000')).toBe('');
    expect(timeSpanText('0.00:00:00')).toBe('');
    expect(timeSpanText('00:00:01')).toBe('1 s');
  });
});

describe('localDateTime', () => {
  const zone = process.env.TZ;
  afterEach(() => {
    process.env.TZ = zone;
  });

  it('gives a UTC time of the server in the local time of the browser', () => {
    process.env.TZ = 'America/Chicago';
    expect(localDateTime('2026-11-18 19:02:06')).toBe('2026-11-18 13:02:06');
    expect(localDateTime('2026-12-01 03:00:00')).toBe('2026-11-30 21:00:00');
    process.env.TZ = 'UTC';
    expect(localDateTime('2026-11-18 19:02:06')).toBe('2026-11-18 19:02:06');
  });

  it('gives back a text in another form as it is', () => {
    expect(localDateTime('')).toBe('');
    expect(localDateTime('soon')).toBe('soon');
    expect(localDateTime('2026-13-45 99:99:99')).toBe('2026-13-45 99:99:99');
  });

  it('reads the day part and drops the fraction', () => {
    expect(timeSpanSeconds('1.00:00:01.5')).toBe(86401);
    expect(timeSpanSeconds('00:10:00')).toBe(600);
  });
});
