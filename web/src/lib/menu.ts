// The menu of tabs and categories, and what the main area shows for an address.

import { findTab, type Route, type SettingsSection } from './route.svelte';
import type { MenuCategory, MenuTab, UnsortedPlace } from './types';

/** The number of the category whose tabs stand in the menu without a heading. */
export const UNSORTED = 0;

export interface MenuGroup {
  category: MenuCategory;
  tabs: MenuTab[];
}

export interface Menu {
  /** The tabs without a heading: those of the category "Unsorted". */
  unsorted: MenuTab[];
  /** Whether the tabs without a heading stand above the categories or below them. */
  unsortedPlace: UnsortedPlace;
  /** The categories that have a tab for this caller, in their order. */
  groups: MenuGroup[];
}

function byPlace<T extends { order: number; id: number }>(first: T, second: T): number {
  return first.order - second.order || first.id - second.id;
}

/**
 * Puts the tabs of a launch answer under their categories. `unsortedPlace` is the
 * `unsortedTabs` setting.
 */
export function buildMenu(
  tabs: MenuTab[],
  categories: MenuCategory[],
  unsortedPlace: UnsortedPlace,
): Menu {
  const sorted = [...tabs].sort(byPlace);
  const numbers = new Set(categories.map((category) => category.category_id));
  const groups = [...categories]
    .sort(byPlace)
    .filter((category) => category.category_id !== UNSORTED)
    .map((category) => ({
      category,
      tabs: sorted.filter((tab) => tab.category_id === category.category_id),
    }))
    .filter((group) => group.tabs.length > 0);
  // A tab whose category does not exist stands with the tabs without a heading.
  const unsorted = sorted.filter(
    (tab) =>
      tab.category_id === UNSORTED || tab.category_id === null || !numbers.has(tab.category_id),
  );
  return { unsorted, unsortedPlace, groups };
}

/** Whether a tab opens in a new browser window and not inside the interface. */
export function opensNewWindow(tab: MenuTab): boolean {
  return tab.type === 2;
}

/**
 * The tab that the start shows: the default tab, or else the first tab of the menu that
 * opens inside the interface.
 */
export function startTab(tabs: MenuTab[]): MenuTab | undefined {
  const inside = tabs.filter((tab) => !opensNewWindow(tab));
  return inside.find((tab) => tab.default === 1) ?? [...inside].sort(byPlace)[0];
}

/** What the main area shows. */
export type Content =
  | { kind: 'login' }
  | { kind: 'register' }
  | { kind: 'recover' }
  | { kind: 'account' }
  | { kind: 'speedTest' }
  | { kind: 'settings'; section: SettingsSection }
  /** The settings, for a caller who may not manage the install. */
  | { kind: 'notAllowed' }
  /** A tab that shows another application in a frame. */
  | { kind: 'frame'; tab: MenuTab }
  | { kind: 'homepage'; tab: MenuTab }
  /** A tab that opens in a new window. The main area offers the link again. */
  | { kind: 'newWindow'; tab: MenuTab }
  /** A tab that has no address to open. */
  | { kind: 'noAddress'; tab: MenuTab }
  /** The address names a tab that this caller does not have. */
  | { kind: 'unknownTab'; name: string }
  /** The caller sees no tab at all. */
  | { kind: 'noTabs' };

export interface Caller {
  loggedIn: boolean;
  /** Whether the caller may manage tabs and categories. */
  canManage: boolean;
}

function contentOfTab(tab: MenuTab, caller: Caller): Content {
  if (tab.type === 0) {
    // A page of Organizr itself. The address names the page.
    if (tab.url?.includes('page/settings')) {
      return caller.canManage ? { kind: 'settings', section: 'tabs' } : { kind: 'notAllowed' };
    }
    return { kind: 'homepage', tab };
  }
  if (!tab.access_url) {
    return { kind: 'noAddress', tab };
  }
  return opensNewWindow(tab) ? { kind: 'newWindow', tab } : { kind: 'frame', tab };
}

/** Works out what the main area shows for an address. */
export function contentFor(route: Route, tabs: MenuTab[], caller: Caller): Content {
  switch (route.page) {
    case 'login':
      return caller.loggedIn ? contentFor({ page: 'home' }, tabs, caller) : { kind: 'login' };
    case 'register':
      return caller.loggedIn ? contentFor({ page: 'home' }, tabs, caller) : { kind: 'register' };
    case 'recover':
      return caller.loggedIn ? contentFor({ page: 'home' }, tabs, caller) : { kind: 'recover' };
    case 'account':
      return caller.loggedIn ? { kind: 'account' } : { kind: 'login' };
    case 'speedTest':
      // The server decides who may run the test, and says so on the page.
      return { kind: 'speedTest' };
    case 'settings':
      if (caller.canManage) {
        return { kind: 'settings', section: route.section };
      }
      return caller.loggedIn ? { kind: 'notAllowed' } : { kind: 'login' };
    case 'tab': {
      const tab = findTab(tabs, route.name);
      if (tab) {
        return contentOfTab(tab, caller);
      }
      // The tab may be one that the caller sees after a login.
      return caller.loggedIn ? { kind: 'unknownTab', name: route.name } : { kind: 'login' };
    }
    case 'home': {
      const tab = startTab(tabs);
      if (tab) {
        return contentOfTab(tab, caller);
      }
      return caller.loggedIn ? { kind: 'noTabs' } : { kind: 'login' };
    }
  }
}

/** The tab that the main area shows, for the mark in the menu. */
export function shownTab(content: Content, route: Route, tabs: MenuTab[]): MenuTab | undefined {
  if ('tab' in content) {
    return content.tab;
  }
  // The tab "Settings" has no content of its own: it leads to the settings.
  if (content.kind === 'settings' || content.kind === 'notAllowed') {
    return route.page === 'tab' ? findTab(tabs, route.name) : undefined;
  }
  return undefined;
}
