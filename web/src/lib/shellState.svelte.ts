// What both shells (the original one and the modern one) share besides the actions of
// `shellActions.ts`: the links at the end of the menu, the check whether the applications
// answer, the buttons for the tab on top, and the media search.

import { api } from './api';
import { frames } from './frames.svelte';
import { opensNewWindow, tabAddress } from './menu';
import { pingInterval, pingList, type PingResult } from './pings';
import { PAGES, router } from './route.svelte';
import { session } from './session.svelte';
import type { MenuTab } from './types';

/** An entry that PHP adds at the end of the menu. */
export interface ExtraLink {
  name: string;
  url: string;
  icon: string;
}

/** The switches of the links that PHP adds at the end of the menu. */
export function menuLinkSwitches() {
  const menu = session.launch?.settings.menuLink ?? {};
  return {
    signout: menu.organizrSignoutMenuLink !== false,
    github: menu.githubMenuLink !== false,
    support: menu.organizrSupportMenuLink !== false,
    docs: menu.organizrDocsMenuLink !== false,
    feature: menu.organizrFeatureRequestLink !== false,
  };
}

/** The links of Organizr that PHP shows to Admins and Co-Admins (`menuExtraTabs()`). */
export function extraLinks(): ExtraLink[] {
  if (!session.canManage) {
    return [];
  }
  const links = menuLinkSwitches();
  return [
    links.github && {
      name: 'Github Repo',
      url: 'https://github.com/causefx/organizr',
      icon: 'oi-fontawesome fa-github',
    },
    links.support && {
      name: 'Organizr Support',
      url: 'https://organizr.app/support',
      icon: 'oi-fontawesome fa-life-ring',
    },
    links.docs && {
      name: 'Organizr Docs',
      url: 'https://docs.organizr.app',
      icon: 'oi-simpleline icon-docs',
    },
    links.feature && {
      name: 'Feature Request',
      url: 'https://feature.organizr.app',
      icon: 'oi-simpleline icon-arrow-up-circle',
    },
  ].filter((link) => link !== false);
}

/**
 * Whether the applications behind the tabs answer. Call `watch` in an effect: the server
 * is asked at once, and then again and again, as long as the menu has a tab with the check
 * switched on.
 */
export class Pings {
  answers = $state<Record<string, PingResult>>({});

  watch(): () => void {
    const list = pingList(session.tabs);
    const every = session.launch ? pingInterval(session.launch) : null;
    if (list.length === 0 || every === null) {
      this.answers = {};
      return () => {};
    }
    const check = async () => {
      try {
        this.answers = (await api<Record<string, PingResult>>('POST', 'ping', { list })).data;
      } catch {
        // Without an answer of the server, nothing is known about the tabs.
        this.answers = {};
      }
    };
    void check();
    const timer = setInterval(() => void check(), every);
    return () => clearInterval(timer);
  }
}

/** The tab on top, when it shows another application in a frame. */
export function framedTab(shown: number | undefined): MenuTab | undefined {
  const tab = session.tabs.find((one) => one.id === shown);
  return tab && tab.type === 1 && !opensNewWindow(tab) && tabAddress(tab) ? tab : undefined;
}

/** Loads what the main area shows again: the frame of a tab, or the page. */
export function reloadShown(shown: number | undefined): void {
  const framed = framedTab(shown);
  if (framed) {
    frames.reload(framed.id);
  } else {
    window.location.reload();
  }
}

/** Closes the frame of the tab on top and goes to the start. */
export function closeShown(shown: number | undefined): void {
  const framed = framedTab(shown);
  if (framed) {
    frames.close(framed.id);
  }
  router.go(PAGES.home);
}

/** Opens what the main area shows in a tab of the browser. */
export function openShown(shown: number | undefined): void {
  const framed = framedTab(shown);
  window.open((framed && tabAddress(framed)) ?? window.location.href, '_blank', 'noopener');
}

export function toggleFullScreen(): void {
  if (document.fullscreenElement) {
    void document.exitFullscreen();
  } else {
    void document.documentElement.requestFullscreen();
  }
}

/** The server of the media search of the top bar, when the caller has the search. */
export function mediaSearchService(): 'plex' | null {
  const search = session.launch?.settings.homepage.search;
  return search?.enabled && search.type === 'plex' ? search.type : null;
}
