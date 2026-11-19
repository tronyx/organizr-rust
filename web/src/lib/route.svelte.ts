// Which page the address names. The interface finds its pages by the part of the
// address after the `#`. A tab has the form of PHP Organizr, `#Plex-Server`, so links and
// bookmarks from there keep working. The pages of the interface itself start with `#/`.

import type { MenuTab } from './types';

export type Route =
  | { page: 'home' }
  | { page: 'login' }
  | { page: 'settings'; section: 'tabs' | 'categories' }
  /** A tab, by its name as the address writes it. */
  | { page: 'tab'; name: string };

/** The part of an address after the `#` for each page of the interface itself. */
export const PAGES = {
  home: '#/',
  login: '#/login',
  tabSettings: '#/settings/tabs',
  categorySettings: '#/settings/categories',
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
    case '/settings':
    case '/settings/tabs':
      return { page: 'settings', section: 'tabs' };
    case '/settings/categories':
      return { page: 'settings', section: 'categories' };
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
