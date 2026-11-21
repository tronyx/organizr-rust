// Which page the address names. The interface finds its pages by the part of the
// address after the `#`. A tab has the form of PHP Organizr, `#Plex-Server`, so links and
// bookmarks from there keep working. The pages of the interface itself start with `#/`.

import type { MenuTab } from './types';

/** The parts of the settings. */
export type SettingsSection =
  | 'general'
  | 'appearance'
  | 'themes'
  | 'tabs'
  | 'categories'
  | 'groups'
  | 'users'
  | 'import'
  | 'login'
  | 'homepage'
  | 'homepage-order'
  | 'plugins'
  | 'plugins-inactive'
  | 'plugin-settings'
  | 'plugin-marketplace'
  | 'theme-settings'
  | 'backups'
  | 'logs'
  | 'updates'
  | 'about'
  | 'donate'
  | 'images';

export type Route =
  | { page: 'home' }
  | { page: 'login' }
  | { page: 'register' }
  /** The page for a person who forgot the password. */
  | { page: 'recover' }
  /** The page of the own account. */
  | { page: 'account' }
  /** The speed test of the plugin. */
  | { page: 'speedTest' }
  /** The invites of the plugin, for the members who make them. */
  | { page: 'invites' }
  /** The chat of the plugin. */
  | { page: 'chat' }
  /** The page for a person with an invite code. */
  | { page: 'useInvite'; code: string }
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
  appearanceSettings: '#/settings/appearance',
  themeSettings: '#/settings/themes',
  tabSettings: '#/settings/tabs',
  categorySettings: '#/settings/categories',
  groupSettings: '#/settings/groups',
  userSettings: '#/settings/users',
  importUserSettings: '#/settings/import',
  loginSettings: '#/settings/login',
  homepageSettings: '#/settings/homepage',
  homepageOrderSettings: '#/settings/homepage/order',
  pluginSettings: '#/settings/plugins',
  inactivePluginSettings: '#/settings/plugins/inactive',
  pluginMarketplaceSettings: '#/settings/plugins/settings',
  pluginMarketplace: '#/settings/plugins/marketplace',
  themeMarketplaceSettings: '#/settings/themes/settings',
  backupSettings: '#/settings/backups',
  logSettings: '#/settings/logs',
  updateSettings: '#/settings/updates',
  aboutSettings: '#/settings/about',
  donateSettings: '#/settings/donate',
  imageSettings: '#/settings/images',
  speedTest: '#/speedtest',
  invites: '#/invites',
  chat: '#/chat',
  useInvite: '#/invite',
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
  const invite = /^\/invite(?:\/([A-Za-z0-9_-]*))?\/*$/.exec(fragment);
  if (invite) {
    return { page: 'useInvite', code: invite[1] ?? '' };
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
    case '/speedtest':
      return { page: 'speedTest' };
    case '/invites':
      return { page: 'invites' };
    case '/chat':
      return { page: 'chat' };
    case '/settings':
    case '/settings/tabs':
      return { page: 'settings', section: 'tabs' };
    case '/settings/general':
      return { page: 'settings', section: 'general' };
    case '/settings/appearance':
      return { page: 'settings', section: 'appearance' };
    case '/settings/themes':
      return { page: 'settings', section: 'themes' };
    case '/settings/categories':
      return { page: 'settings', section: 'categories' };
    case '/settings/groups':
      return { page: 'settings', section: 'groups' };
    case '/settings/users':
      return { page: 'settings', section: 'users' };
    case '/settings/import':
      return { page: 'settings', section: 'import' };
    case '/settings/login':
    case '/settings/sso':
      return { page: 'settings', section: 'login' };
    case '/settings/updates':
      return { page: 'settings', section: 'updates' };
    case '/settings/about':
      return { page: 'settings', section: 'about' };
    case '/settings/donate':
      return { page: 'settings', section: 'donate' };
    case '/settings/images':
      return { page: 'settings', section: 'images' };
    case '/settings/homepage':
      return { page: 'settings', section: 'homepage' };
    case '/settings/homepage/order':
      return { page: 'settings', section: 'homepage-order' };
    case '/settings/plugins':
    case '/settings/mail':
      return { page: 'settings', section: 'plugins' };
    case '/settings/plugins/inactive':
      return { page: 'settings', section: 'plugins-inactive' };
    case '/settings/plugins/settings':
      return { page: 'settings', section: 'plugin-settings' };
    case '/settings/plugins/marketplace':
      return { page: 'settings', section: 'plugin-marketplace' };
    case '/settings/themes/settings':
      return { page: 'settings', section: 'theme-settings' };
    case '/settings/backups':
      return { page: 'settings', section: 'backups' };
    case '/settings/logs':
      return { page: 'settings', section: 'logs' };
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

/**
 * The page for the address that the browser opened. The e-mail of an invite leads to
 * the site with `?invite=` and the code, and that opens the page for an invite code.
 */
export function firstRoute(hash: string, search: string): Route {
  const code = /[?&]invite=([^&#]*)/.exec(search)?.[1];
  if (code !== undefined && hash.replace(/^#\/?/, '') === '') {
    return parseRoute(`#/invite/${code}`);
  }
  return parseRoute(hash);
}

class Router {
  route = $state<Route>(firstRoute(window.location.hash, window.location.search));

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
