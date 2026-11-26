import { describe, expect, it } from 'vitest';

import { DEFAULT_REFRESH, minutesOfRefresh, SHORTEST_REFRESH } from './itemData.svelte';
import {
  dollars,
  donateOptions,
  frameAddress,
  frameSandbox,
  jellyStatOptions,
  jellyStatPoster,
  liveTvOptions,
  momentText,
} from './statistics';

describe('the options of the JellyStat item', () => {
  it('are what the server sends', () => {
    const options = jellyStatOptions({
      mode: 'iframe',
      url: 'https://stats.example.com',
      frameHeight: 600,
      frameScrolling: false,
      refreshMinutes: 10,
      days: 7,
      maxItems: 3,
      showLibraries: false,
      showUsers: false,
      showMostWatched: false,
      showMovies: false,
      showShows: false,
      showMusic: false,
    });

    expect(options.mode).toBe('iframe');
    expect(options.url).toBe('https://stats.example.com');
    expect(options.frameHeight).toBe(600);
    expect(options.frameScrolling).toBe(false);
    expect(options.refreshMinutes).toBe(10);
    expect(options.maxItems).toBe(3);
    expect(options.showLibraries).toBe(false);
    expect(options.showMusic).toBe(false);
  });

  it('have the defaults of PHP Organizr for what the server does not send', () => {
    expect(jellyStatOptions(undefined)).toEqual({
      mode: 'native',
      url: '',
      frameHeight: 800,
      frameScrolling: true,
      refreshMinutes: 5,
      days: 30,
      maxItems: 10,
      showLibraries: true,
      showUsers: true,
      showMostWatched: true,
      showMovies: true,
      showShows: true,
      showMusic: true,
    });
    expect(jellyStatOptions({ mode: 'other', frameHeight: 'tall' }).mode).toBe('native');
    expect(jellyStatOptions({ frameHeight: 'tall' }).frameHeight).toBe(800);
  });
});

describe('the options of the live television item and of the donate item', () => {
  it('are what the server sends, or the defaults', () => {
    expect(liveTvOptions({ compact: true, showUserInfo: true, refreshMinutes: '2' })).toEqual({
      refreshMinutes: 2,
      compact: true,
      showDuration: true,
      showSeriesInfo: true,
      showUserInfo: true,
      showCompleted: true,
    });
    expect(liveTvOptions(undefined).showUserInfo).toBe(false);

    expect(
      donateOptions({ heading: 'Help', description: 'Please', minimum: 2.5, history: true }),
    ).toEqual({ heading: 'Help', description: 'Please', minimum: 2.5, history: true });
    expect(donateOptions(undefined)).toEqual({
      heading: '',
      description: '',
      minimum: 1,
      history: false,
    });
    expect(donateOptions({ minimum: 'much' }).minimum).toBe(1);
  });
});

describe('minutesOfRefresh', () => {
  it('is the minutes in milliseconds, and never shorter than the shortest time', () => {
    expect(minutesOfRefresh(5)).toBe(300000);
    expect(minutesOfRefresh(0.01)).toBe(SHORTEST_REFRESH);
    expect(minutesOfRefresh(0)).toBe(DEFAULT_REFRESH);
    expect(minutesOfRefresh(Number.NaN)).toBe(DEFAULT_REFRESH);
  });
});

describe('momentText', () => {
  it('writes a moment as a date and a time, and keeps a text that is no moment', () => {
    expect(momentText('2026-11-20T10:00:00Z')).toContain('2026');
    expect(momentText('next week')).toBe('next week');
    expect(momentText(null)).toBe('');
    expect(momentText('')).toBe('');
  });
});

describe('dollars', () => {
  it('writes an amount of money', () => {
    expect(dollars(12.5)).toBe('$12.50');
    expect(dollars(5)).toBe('$5.00');
  });
});

describe('jellyStatPoster', () => {
  it('is the address of the picture that JellyStat passes on', () => {
    expect(jellyStatPoster('https://stats.example.com/', 'abc-123')).toBe(
      'https://stats.example.com/proxy/Items/Images/Primary?id=abc-123&fillWidth=200&quality=90',
    );
    expect(jellyStatPoster('https://stats.example.com', 815)).toContain('id=815');
  });

  it('is nothing without an address, and for an id that is none', () => {
    expect(jellyStatPoster('', 'abc')).toBeNull();
    expect(jellyStatPoster('https://stats.example.com', 'The Show')).toBeNull();
    expect(jellyStatPoster('https://stats.example.com', null)).toBeNull();
    expect(jellyStatPoster('https://stats.example.com', '../x')).toBeNull();
  });
});

describe('frameAddress', () => {
  it('takes a web address and a path of this site only', () => {
    expect(frameAddress('https://stats.example.com')).toBe('https://stats.example.com');
    expect(frameAddress('/jellystat')).toBe('/jellystat');
    expect(frameAddress('javascript:alert(1)')).toBeNull();
    expect(frameAddress('')).toBeNull();
  });
});

describe('frameSandbox', () => {
  const origin = 'https://organizr.example.com';

  it('keeps the frame of another site on its own origin', () => {
    expect(frameSandbox('https://stats.example.com', origin)).toBe(
      'allow-scripts allow-same-origin allow-forms allow-popups',
    );
  });

  it('gives a frame of this site no access to Organizr', () => {
    for (const address of ['/jellystat', 'https://organizr.example.com/jellystat/']) {
      expect(frameSandbox(address, origin), address).toBe('allow-scripts allow-forms allow-popups');
    }
  });
});
