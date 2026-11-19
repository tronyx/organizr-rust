// Which page the address names. The interface finds its pages by the part of the
// address after the `#`. A tab has the form of PHP Organizr, `#Plex-Server`, so links and
// bookmarks from there keep working. The pages of the interface itself start with `#/`.

import type { MenuTab } from './types';

/** The parts of the settings. */
export type SettingsSection =
  'general' | 'tabs' | 'categories' | 'groups' | 'users' | 'mail' | 'login' | 'homepage';

export type Route =
  | { page: 'home' }
  | { page: 'login' }
  | { page: 'register' }
  /** The page for a person who forgot the password. */
  | { page: 'recover' }
  /** The page of the own account. */
  | { page: 'account' }
  | { page: 'settings'; section: SettingsSection }
  /** A tab, by its name as the address writes it. */
  | { page: 'tab'; name: string };

/** The part of an address after the `#` for each page of the interface itself. */
export const PAGES = {
  home: '#/',
  login: '#/login',
  register: '#/register',
  recover: '#/recover',
  account: '#/account',
  generalSettings: '#/settings/general',
  tabSettings: '#/settings/tabs',
  categorySettings: '#/settings/categories',
  groupSettings: '#/settings/groups',
  userSettings: '#/settings/users',
  mailSettings: '#/settings/mail',
  loginSettings: '#/settings/login',
  homepageSettings: '#/settings/homepage',
} as const;

/** Reads the part of an address from the `#` on. */
export function parseRoute(hash: string): Route {
  const fragment = hash.replace(/^#/, '');
  if (fragment === '' || fragment === '/') {
    return { page: 'home' };
  }
  if (!fragment.startsWith('/')) {
    let name = fragment;
    try {
      name = decodeURIComponent(fragment);
    } catch {
      // Not a valid encoding. The text is taken as it is.
    }
    return { page: 'tab', name };
  }
  switch (fragment.replace(/\/+$/, '')) {
    case '/login':
      return { page: 'login' };
    case '/register':
      return { page: 'register' };
    case '/recover':
      return { page: 'recover' };
    case '/account':
      return { page: 'account' };
    case '/settings':
    case '/settings/tabs':
      return { page: 'settings', section: 'tabs' };
    case '/settings/general':
      return { page: 'settings', section: 'general' };
    case '/settings/categories':
      return { page: 'settings', section: 'categories' };
    case '/settings/groups':
      return { page: 'settings', section: 'groups' };
    case '/settings/users':
      return { page: 'settings', section: 'users' };
    case '/settings/mail':
      return { page: 'settings', section: 'mail' };
    case '/settings/login':
      return { page: 'settings', section: 'login' };
    case '/settings/homepage':
      return { page: 'settings', section: 'homepage' };
    default:
      return { page: 'home' };
  }
}

/** The name of a tab as an address writes it: a dash stands for each space. */
export function tabAddressName(name: string): string {
  return name.replaceAll(' ', '-');
}

/** The part of an address from the `#` on that opens a tab. */
export function tabHash(name: string): string {
  return `#${encodeURI(tabAddressName(name))}`;
}

/** The tab that the name in an address stands for. */
export function findTab<T extends Pick<MenuTab, 'name'>>(tabs: T[], name: string): T | undefined {
  const wanted = tabAddressName(name);
  return (
    tabs.find((tab) => tabAddressName(tab.name) === wanted) ??
    tabs.find((tab) => tabAddressName(tab.name).toLowerCase() === wanted.toLowerCase())
  );
}

class Router {
  route = $state<Route>(parseRoute(window.location.hash));

  constructor() {
    window.addEventListener('hashchange', () => {
      this.route = parseRoute(window.location.hash);
    });
  }

  /** Goes to a page: `hash` is the part of the address from the `#` on. */
  go(hash: string): void {
    window.location.hash = hash;
    // The browser reports the change a moment later. The pages follow at once.
    this.route = parseRoute(hash);
  }
}

export const router = new Router();
