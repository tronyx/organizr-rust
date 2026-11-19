import { describe, expect, it } from 'vitest';

import { pingInterval, pingList, pingStatus, SHORTEST_INTERVAL } from './pings';
import type { Launch, MenuTab } from './types';

function tab(id: number, changes: Partial<MenuTab>): MenuTab {
  return {
    id,
    order: id,
    category_id: 0,
    name: `Tab ${id}`,
    url: 'https://example.com/',
    url_local: '',
    default: 0,
    enabled: 1,
    group_id: 4,
    group_id_max: 0,
    add_to_admin: 0,
    image: null,
    type: 1,
    splash: 0,
    ping: 0,
    ping_url: null,
    timeout: 0,
    timeout_ms: null,
    preload: 0,
    access_url: 'https://example.com/',
    ...changes,
  };
}

function launch(groupID: number | null, auth: string, admin = '60000', other = '600000'): Launch {
  return {
    categories: [],
    tabs: [],
    user:
      groupID === null
        ? null
        : {
            username: 'zoe',
            group: 'User',
            groupID,
            email: null,
            image: null,
            userID: 3,
            loggedin: true,
          },
    version: '0.1.0',
    appearance: { title: 'Organizr' },
    plugins: {},
    settings: {
      misc: { unsortedTabs: 'top' },
      ping: { auth, adminRefresh: admin, everyoneRefresh: other },
    },
    status: { status: 'ok' },
  };
}

describe('pingList', () => {
  it('holds the address of each tab with the check on, once', () => {
    const tabs = [
      tab(1, { ping: 1, ping_url: ' plex.example.com:32400 ' }),
      tab(2, { ping: 1, ping_url: 'plex.example.com:32400' }),
      tab(3, { ping: 0, ping_url: 'off.example.com' }),
      tab(4, { ping: 1, ping_url: '' }),
      tab(5, { ping: 1, ping_url: null }),
      tab(6, { ping: 1, ping_url: '192.168.1.5' }),
    ];

    expect(pingList(tabs)).toEqual(['plex.example.com:32400', '192.168.1.5']);
    expect(pingList([])).toEqual([]);
  });
});

describe('pingInterval', () => {
  it('is the interval of the setting for the level of the caller', () => {
    expect(pingInterval(launch(0, '1'))).toBe(60000);
    expect(pingInterval(launch(1, '1'))).toBe(60000);
    expect(pingInterval(launch(4, '999'))).toBe(600000);
    expect(pingInterval(launch(null, '999'))).toBe(600000);
  });

  it('is nothing for a caller who may not ask', () => {
    expect(pingInterval(launch(2, '1'))).toBeNull();
    expect(pingInterval(launch(null, '998'))).toBeNull();
  });

  it('is never shorter than five seconds, and nothing for a setting that is no number', () => {
    expect(pingInterval(launch(0, '1', '100'))).toBe(SHORTEST_INTERVAL);
    expect(pingInterval(launch(0, '1', 'often'))).toBeNull();
  });
});

describe('pingStatus', () => {
  const results = { 'plex.example.com:32400': 12.5, 'down.example.com': false as const, zero: 0 };

  it('says whether the address of the tab answered', () => {
    expect(pingStatus(tab(1, { ping: 1, ping_url: 'plex.example.com:32400' }), results)).toBe('up');
    expect(pingStatus(tab(1, { ping: 1, ping_url: ' down.example.com ' }), results)).toBe('down');
    // An answer after no time at all is an answer.
    expect(pingStatus(tab(1, { ping: 1, ping_url: 'zero' }), results)).toBe('up');
  });

  it('says nothing for a tab without a check, or without an answer yet', () => {
    expect(pingStatus(tab(1, { ping: 0, ping_url: 'plex.example.com:32400' }), results)).toBe(
      undefined,
    );
    expect(pingStatus(tab(1, { ping: 1, ping_url: 'other.example.com' }), results)).toBe(undefined);
    expect(pingStatus(tab(1, { ping: 1, ping_url: null }), results)).toBe(undefined);
    expect(pingStatus(tab(1, { ping: 1, ping_url: 'toString' }), {})).toBe(undefined);
  });
});
