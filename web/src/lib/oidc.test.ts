import { afterEach, describe, expect, it } from 'vitest';

import { afterLogout, authorizeAddress, NO_REDIRECT, providerList, redirectTarget } from './oidc';

const ON = { enabled: true, autoRedirect: 'keycloak', logoutURL: '' };

afterEach(() => {
  window.sessionStorage.clear();
});

describe('the list of providers', () => {
  it('reads the object of the server, and the empty list for no provider', () => {
    expect(providerList([])).toEqual([]);
    expect(providerList(null)).toEqual([]);
    expect(
      providerList({
        keycloak: { name: 'Company login', enabled: true },
        zitadel: { name: '', enabled: true },
      }),
    ).toEqual([
      { key: 'keycloak', name: 'Company login' },
      { key: 'zitadel', name: 'zitadel' },
    ]);
  });

  it('starts a login at the route of the server', () => {
    expect(authorizeAddress('keycloak', 'https://organizr.example.com/')).toBe(
      'https://organizr.example.com/api/v2/oidc/keycloak/authorize',
    );
    expect(authorizeAddress('a/b', 'https://organizr.example.com/')).toBe(
      'https://organizr.example.com/api/v2/oidc/a%2Fb/authorize',
    );
  });
});

describe('the automatic redirect', () => {
  it('goes to the provider of the settings', () => {
    expect(redirectTarget(ON, '')).toBe('keycloak');
    expect(redirectTarget(ON, '#/login')).toBe('keycloak');
  });

  it('stays with #noredirect, without a provider, or with the login switched off', () => {
    expect(redirectTarget(ON, '#noredirect')).toBeNull();
    expect(redirectTarget({ ...ON, autoRedirect: '' }, '')).toBeNull();
    expect(redirectTarget({ ...ON, enabled: false }, '')).toBeNull();
    expect(redirectTarget(undefined, '')).toBeNull();
  });

  it('stays after a logout in the same tab', () => {
    expect(afterLogout(ON)).toBeNull();
    expect(window.sessionStorage.getItem(NO_REDIRECT)).toBe('1');
    expect(redirectTarget(ON, '')).toBeNull();
  });
});

describe('the logout', () => {
  it('goes to the address of the settings, when it is a web address', () => {
    expect(afterLogout({ ...ON, logoutURL: 'https://id.example.com/logout' })).toBe(
      'https://id.example.com/logout',
    );
    expect(afterLogout({ ...ON, logoutURL: 'javascript:alert(1)' })).toBeNull();
    expect(afterLogout(undefined)).toBeNull();
  });

  it('keeps the redirect when the settings have none', () => {
    afterLogout({ enabled: true, autoRedirect: '', logoutURL: '' });
    expect(window.sessionStorage.getItem(NO_REDIRECT)).toBeNull();
  });
});
