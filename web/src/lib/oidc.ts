// The login with an OpenID Connect provider, such as Authentik, Keycloak, PocketID or
// Zitadel.
//
// The browser goes to `api/v2/oidc/<provider>/authorize` on the server, which sends it on
// to the login page of the provider. The provider sends it back to the server, which
// starts the session and sends the browser to the start page. The interface itself only
// shows the buttons, and goes to the provider by itself when the settings say so.

import { apiUrl } from './api';
import type { OidcLaunch } from './types';

/** A provider that the server has switched on, as `GET oidc/providers` names it. */
export interface OidcProvider {
  /** The key in the address, for example `keycloak`. */
  key: string;
  /** The name that the button shows. */
  name: string;
}

/**
 * Under this name the browser keeps, for the tab, that the person does not want to go to
 * the provider by itself. PHP Organizr reads the same name.
 */
export const NO_REDIRECT = 'oidc_no_redirect';

/**
 * The providers of the answer of `GET oidc/providers`. The server sends an empty list for
 * no provider, and an object from the key to the name for some, as PHP Organizr does.
 */
export function providerList(data: unknown): OidcProvider[] {
  if (typeof data !== 'object' || data === null || Array.isArray(data)) {
    return [];
  }
  return Object.entries(data as Record<string, unknown>).flatMap(([key, value]) => {
    const name = (value as { name?: unknown } | null)?.name;
    return [{ key, name: typeof name === 'string' && name !== '' ? name : key }];
  });
}

/** Sends the browser to another address. A test replaces it, because jsdom cannot go. */
export const browser = {
  go(address: string): void {
    window.location.assign(address);
  },
};

/** The address that starts a login at a provider. */
export function authorizeAddress(key: string, page?: string): string {
  return apiUrl(`oidc/${encodeURIComponent(key)}/authorize`, page);
}

/**
 * The API path that starts the link of the account of the session to a provider. Only a
 * page of this site can start it, with the password of the account. The answer holds the
 * address of the login page of the provider, which the browser then goes to.
 */
export function linkPath(key: string): string {
  return `oidc/${encodeURIComponent(key)}/link`;
}

/** The provider of a link that just finished, from the query of the page, or `null`. */
export function linkedNotice(search: string): string | null {
  const key = new URLSearchParams(search).get('oidcLinked');
  return key === null || key === '' ? null : key;
}

/** Whether the browser keeps the wish to stay on the login page. */
function staysHere(): boolean {
  try {
    return window.sessionStorage.getItem(NO_REDIRECT) !== null;
  } catch {
    return false;
  }
}

/**
 * The provider that the login page goes to by itself, or `null`: the one of the settings,
 * unless the address holds `noredirect` (as `#noredirect`) or the person logged out in
 * this tab.
 */
export function redirectTarget(launch: OidcLaunch | undefined, hash: string): string | null {
  const provider = launch?.enabled ? launch.autoRedirect : '';
  if (!provider || hash.includes('noredirect') || staysHere()) {
    return null;
  }
  return provider;
}

/**
 * After a logout: keeps the login page from sending the person straight back to the
 * provider, which would log the person in again. Returns the address of the settings to go
 * to, or `null`.
 */
export function afterLogout(launch: OidcLaunch | undefined): string | null {
  if (launch?.autoRedirect) {
    try {
      window.sessionStorage.setItem(NO_REDIRECT, '1');
    } catch {
      // The browser keeps nothing. The login page then goes to the provider again.
    }
  }
  const address = launch?.logoutURL ?? '';
  return /^https?:\/\//i.test(address) ? address : null;
}
