import { describe, expect, it } from 'vitest';

import { buildMenu, contentFor, pictureOf, shownTab, startTab, type Caller } from './menu';
import type { MenuCategory, MenuTab } from './types';

function tab(id: number, name: string, changes: Partial<MenuTab> = {}): MenuTab {
  return {
    id,
    order: id,
    category_id: 0,
    name,
    url: `https://${name.toLowerCase()}.example.com/`,
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
    access_url: `https://${name.toLowerCase()}.example.com/`,
    ...changes,
  };
}

function category(id: number, name: string, number: number, order = id): MenuCategory {
  return { id, order, category: name, category_id: number, image: null, default: 0, count: 0 };
}

const GUEST: Caller = { loggedIn: false, canManage: false };
const USER: Caller = { loggedIn: true, canManage: false };
const ADMIN: Caller = { loggedIn: true, canManage: true };

const SETTINGS = tab(1, 'Settings', { type: 0, url: 'api/v2/page/settings', access_url: null });
const HOMEPAGE = tab(2, 'Homepage', { type: 0, url: 'api/v2/page/homepage', access_url: null });

describe('buildMenu', () => {
  const categories = [
    category(1, 'Unsorted', 0),
    category(2, 'Media', 7, 3),
    category(3, 'Downloads', 8, 2),
    category(4, 'Empty', 9),
  ];

  it('puts each tab under its category, in the order of the places', () => {
    // The server sends the tabs in downward order when tabs without a heading go on top.
    const tabs = [
      tab(5, 'Sonarr', { order: 9, category_id: 8 }),
      tab(4, 'Plex', { order: 5, category_id: 7 }),
      tab(3, 'Jellyfin', { order: 5, category_id: 7 }),
      tab(2, 'Status', { order: 2 }),
      tab(1, 'Welcome', { order: 1 }),
    ];

    const menu = buildMenu(tabs, categories);

    expect(menu.unsorted.map((entry) => entry.name)).toEqual(['Welcome', 'Status']);
    expect(menu.unsortedPlace).toBe('top');
    // "Downloads" stands before "Media", and a category without a tab is left out.
    expect(
      menu.groups.map((group) => [group.category.category, group.tabs.map((e) => e.name)]),
    ).toEqual([
      ['Downloads', ['Sonarr']],
      // Two tabs on one place follow their id.
      ['Media', ['Jellyfin', 'Plex']],
    ]);
  });

  it('puts the tabs without a heading below when the server sends the tabs upwards', () => {
    const tabs = [tab(1, 'Welcome'), tab(4, 'Plex', { category_id: 7 })];

    expect(buildMenu(tabs, categories).unsortedPlace).toBe('bottom');
    expect(buildMenu([tab(1, 'Welcome')], categories).unsortedPlace).toBe('top');
    expect(buildMenu([], categories).unsortedPlace).toBe('top');
  });

  it('keeps a tab whose category does not exist', () => {
    const tabs = [tab(1, 'Lost', { category_id: 55 }), tab(2, 'None', { category_id: null })];

    const menu = buildMenu(tabs, categories);

    expect(menu.unsorted.map((entry) => entry.name)).toEqual(['Lost', 'None']);
    expect(menu.groups).toEqual([]);
  });
});

describe('startTab', () => {
  it('is the default tab', () => {
    const tabs = [tab(1, 'Plex'), tab(2, 'Sonarr', { default: 1 })];

    expect(startTab(tabs)?.name).toBe('Sonarr');
  });

  it('is else the first tab that opens inside the interface', () => {
    const tabs = [
      tab(3, 'Radarr', { order: 3 }),
      tab(2, 'Sonarr', { order: 2 }),
      tab(1, 'Docs', { order: 1, type: 2 }),
    ];

    expect(startTab(tabs)?.name).toBe('Sonarr');
    expect(startTab([tab(1, 'Docs', { type: 2, default: 1 })])).toBeUndefined();
    expect(startTab([])).toBeUndefined();
  });
});

describe('contentFor', () => {
  const plex = tab(3, 'Plex Server');
  const docs = tab(4, 'Docs', { type: 2 });
  const broken = tab(5, 'Broken', { access_url: '' });
  const tabs = [SETTINGS, HOMEPAGE, plex, docs, broken];

  it('shows the tab that the address names', () => {
    expect(contentFor({ page: 'tab', name: 'Plex-Server' }, tabs, USER)).toEqual({
      kind: 'frame',
      tab: plex,
    });
    expect(contentFor({ page: 'tab', name: 'Docs' }, tabs, USER)).toEqual({
      kind: 'newWindow',
      tab: docs,
    });
    expect(contentFor({ page: 'tab', name: 'Homepage' }, tabs, USER)).toEqual({
      kind: 'homepage',
      tab: HOMEPAGE,
    });
    expect(contentFor({ page: 'tab', name: 'Broken' }, tabs, USER)).toEqual({
      kind: 'noAddress',
      tab: broken,
    });
  });

  it('shows the start tab for an address without a page', () => {
    expect(contentFor({ page: 'home' }, [plex, docs], USER)).toEqual({ kind: 'frame', tab: plex });
    expect(contentFor({ page: 'home' }, [], USER)).toEqual({ kind: 'noTabs' });
  });

  it('sends a guest to the login when there is nothing to show', () => {
    expect(contentFor({ page: 'home' }, [], GUEST)).toEqual({ kind: 'login' });
    expect(contentFor({ page: 'tab', name: 'Plex-Server' }, [], GUEST)).toEqual({ kind: 'login' });
    expect(contentFor({ page: 'settings', section: 'tabs' }, tabs, GUEST)).toEqual({
      kind: 'login',
    });
    expect(contentFor({ page: 'login' }, tabs, GUEST)).toEqual({ kind: 'login' });
  });

  it('shows a guest the tabs that guests have', () => {
    expect(contentFor({ page: 'home' }, [plex], GUEST)).toEqual({ kind: 'frame', tab: plex });
  });

  it('shows the page for a new account to a guest', () => {
    expect(contentFor({ page: 'register' }, tabs, GUEST)).toEqual({ kind: 'register' });
  });

  it('does not show the login page to a user who is logged in', () => {
    expect(contentFor({ page: 'login' }, [plex], USER)).toEqual({ kind: 'frame', tab: plex });
    expect(contentFor({ page: 'register' }, [plex], USER)).toEqual({
      kind: 'frame',
      tab: plex,
    });
  });

  it('says so when a user who is logged in names a tab that is not there', () => {
    expect(contentFor({ page: 'tab', name: 'Radarr' }, tabs, USER)).toEqual({
      kind: 'unknownTab',
      name: 'Radarr',
    });
  });

  it('shows the settings only to a caller who may manage the install', () => {
    const route = { page: 'settings', section: 'categories' } as const;

    expect(contentFor(route, tabs, ADMIN)).toEqual({ kind: 'settings', section: 'categories' });
    expect(contentFor(route, tabs, USER)).toEqual({ kind: 'notAllowed' });
    // The tab "Settings" leads to the same place.
    expect(contentFor({ page: 'tab', name: 'Settings' }, tabs, ADMIN)).toEqual({
      kind: 'settings',
      section: 'tabs',
    });
    expect(contentFor({ page: 'tab', name: 'Settings' }, tabs, USER)).toEqual({
      kind: 'notAllowed',
    });
  });
});

describe('shownTab', () => {
  const plex = tab(3, 'Plex');
  const tabs = [SETTINGS, plex];

  it('is the tab of the content', () => {
    const route = { page: 'tab', name: 'Plex' } as const;

    expect(shownTab(contentFor(route, tabs, ADMIN), route, tabs)).toBe(plex);
  });

  it('is the tab "Settings" when the address names it, and no tab for the settings alone', () => {
    const byTab = { page: 'tab', name: 'Settings' } as const;
    const byPage = { page: 'settings', section: 'tabs' } as const;

    expect(shownTab(contentFor(byTab, tabs, ADMIN), byTab, tabs)).toBe(SETTINGS);
    expect(shownTab(contentFor(byPage, tabs, ADMIN), byPage, tabs)).toBeUndefined();
    expect(shownTab({ kind: 'login' }, { page: 'login' }, tabs)).toBeUndefined();
  });
});

describe('pictureOf', () => {
  it('shows a picture that the browser can load', () => {
    expect(pictureOf('https://example.com/plex.png', 'Plex')).toEqual({
      kind: 'image',
      address: 'https://example.com/plex.png',
    });
    expect(pictureOf('data:image/png;base64,AAAA', 'Plex').kind).toBe('image');
  });

  it('shows the first letter of the name for everything else', () => {
    for (const image of [null, '', 'fontawesome::cog', 'plugins/images/tabs/plex.png']) {
      expect(pictureOf(image, ' plex')).toEqual({ kind: 'letter', letter: 'P' });
    }
    expect(pictureOf(null, 'émile')).toEqual({ kind: 'letter', letter: 'É' });
    expect(pictureOf(null, '  ')).toEqual({ kind: 'letter', letter: '?' });
    // A script is no picture.
    expect(pictureOf('javascript:alert(1)', 'x').kind).toBe('letter');
  });
});
