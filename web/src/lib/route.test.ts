import { describe, expect, it } from 'vitest';

import { findTab, PAGES, parseRoute, router, tabHash } from './route.svelte';

describe('parseRoute', () => {
  it('takes an address without a page for the start', () => {
    for (const hash of ['', '#', '#/']) {
      expect(parseRoute(hash)).toEqual({ page: 'home' });
    }
  });

  it('knows the pages of the interface itself', () => {
    expect(parseRoute(PAGES.login)).toEqual({ page: 'login' });
    expect(parseRoute('#/login/')).toEqual({ page: 'login' });
    expect(parseRoute(PAGES.register)).toEqual({ page: 'register' });
    expect(parseRoute(PAGES.tabSettings)).toEqual({ page: 'settings', section: 'tabs' });
    expect(parseRoute('#/settings')).toEqual({ page: 'settings', section: 'tabs' });
    expect(parseRoute(PAGES.categorySettings)).toEqual({
      page: 'settings',
      section: 'categories',
    });
    expect(parseRoute(PAGES.groupSettings)).toEqual({ page: 'settings', section: 'groups' });
    expect(parseRoute(PAGES.userSettings)).toEqual({ page: 'settings', section: 'users' });
    expect(parseRoute(PAGES.generalSettings)).toEqual({ page: 'settings', section: 'general' });
    expect(parseRoute('#/settings/mail')).toEqual({ page: 'settings', section: 'plugins' });
    expect(parseRoute(PAGES.loginSettings)).toEqual({ page: 'settings', section: 'login' });
    expect(parseRoute(PAGES.signInSettings)).toEqual({ page: 'settings', section: 'sign-in' });
    expect(parseRoute(PAGES.loginSecuritySettings)).toEqual({
      page: 'settings',
      section: 'login-security',
    });
    expect(parseRoute(PAGES.generalOtherSettings)).toEqual({
      page: 'settings',
      section: 'general-other',
    });
    expect(parseRoute(PAGES.recover)).toEqual({ page: 'recover' });
    expect(parseRoute(PAGES.account)).toEqual({ page: 'account' });
  });

  it('takes a page that it does not know for the start', () => {
    expect(parseRoute('#/nothing/here')).toEqual({ page: 'home' });
  });

  it('takes every other text for the name of a tab, as PHP Organizr writes it', () => {
    expect(parseRoute('#Plex')).toEqual({ page: 'tab', name: 'Plex' });
    expect(parseRoute('#Plex-Server')).toEqual({ page: 'tab', name: 'Plex-Server' });
    expect(parseRoute('#Caf%C3%A9')).toEqual({ page: 'tab', name: 'Café' });
    // Text that is not a valid encoding is taken as it is.
    expect(parseRoute('#100%')).toEqual({ page: 'tab', name: '100%' });
  });
});

describe('tabHash', () => {
  it('writes a dash for each space, and encodes the rest', () => {
    expect(tabHash('Plex')).toBe('#Plex');
    expect(tabHash('Plex Media Server')).toBe('#Plex-Media-Server');
    expect(tabHash('Café')).toBe('#Caf%C3%A9');
  });

  it('gives an address that leads back to the tab', () => {
    const tabs = [{ name: 'Plex Media Server' }, { name: 'Café' }, { name: 'Sonarr' }];
    for (const tab of tabs) {
      const route = parseRoute(tabHash(tab.name));

      expect(route.page === 'tab' ? findTab(tabs, route.name) : undefined).toBe(tab);
    }
  });
});

describe('findTab', () => {
  const tabs = [{ name: 'Plex Server' }, { name: 'plex' }, { name: 'Sonarr-4K' }];

  it('takes a dash for a space or for a dash', () => {
    expect(findTab(tabs, 'Plex-Server')).toBe(tabs[0]);
    expect(findTab(tabs, 'Plex Server')).toBe(tabs[0]);
    expect(findTab(tabs, 'Sonarr-4K')).toBe(tabs[2]);
  });

  it('prefers the same letter case, and else takes another one', () => {
    expect(findTab(tabs, 'plex')).toBe(tabs[1]);
    expect(findTab(tabs, 'PLEX')).toBe(tabs[1]);
    expect(findTab(tabs, 'sonarr-4k')).toBe(tabs[2]);
  });

  it('finds nothing for a name that no tab has', () => {
    expect(findTab(tabs, 'Radarr')).toBeUndefined();
  });
});

describe('router', () => {
  it('follows the address of the page', () => {
    router.go(PAGES.login);
    expect(router.route).toEqual({ page: 'login' });
    expect(window.location.hash).toBe('#/login');

    window.location.hash = '#Plex';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(router.route).toEqual({ page: 'tab', name: 'Plex' });
  });
});
