// What both shells (the original one and the modern one) do for the person: log out, lock
// the account, and find out which plugin pages the person may open.

import { api } from './api';
import { browserStorage, clearJellyfinLogin } from './jellyfinSso';
import { afterLogout, browser } from './oidc';
import { PAGES, router } from './route.svelte';
import { session } from './session.svelte';

/** Logs out, then loads the session again. A failure is thrown for the caller to show. */
export async function logOut(): Promise<void> {
  const oidc = session.launch?.settings.login?.oidc;
  const jellyfin = session.launch?.settings.sso.jellyfin;
  await api('GET', 'logout');
  clearJellyfinLogin(jellyfin, location.origin, browserStorage);
  const away = afterLogout(oidc);
  if (away !== null) {
    browser.go(away);
    return;
  }
  await session.load();
  router.go(PAGES.home);
}

/** Locks the account: the interface then shows only the lock screen. */
export async function lockAccount(): Promise<void> {
  await api('POST', 'users/lock');
  await session.load();
}

/** Whether a plugin is on, and this person has its level. `name` is `SPEEDTEST`. */
export function canUse(name: string): boolean {
  const plugins = session.launch?.plugins;
  const group = session.launch?.user?.groupID;
  if (plugins?.[`${name}-enabled`] !== true || group === undefined) {
    return false;
  }
  const includes = plugins.includes as Record<string, unknown> | undefined;
  const level = Number(includes?.[`${name}-Auth-include`]);
  return Number.isFinite(level) ? group <= level : group <= 1;
}
