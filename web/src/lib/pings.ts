// The check whether the application behind a tab answers: which addresses to ask the
// server about, how often, and what to show for an answer.

import type { Launch, MenuTab } from './types';

/** The answer of the server for one address: the time in milliseconds, or `false`. */
export type PingResult = number | false;

/** The shortest time between two checks, in milliseconds, whatever the setting says. */
export const SHORTEST_INTERVAL = 5000;

/** The level of a Co-Admin. Callers of this level, or more access, use the shorter interval. */
const CO_ADMIN = 1;

/** The addresses to check: the one of each tab with the check switched on, once each. */
export function pingList(tabs: MenuTab[]): string[] {
  const addresses = tabs
    .filter((tab) => tab.ping === 1)
    .map((tab) => tab.ping_url?.trim() ?? '')
    .filter((address) => address !== '');
  return [...new Set(addresses)];
}

/**
 * How long to wait between two checks, in milliseconds, or `null` when the server does
 * not check for this caller: the `pingAuth` setting names the level that may ask.
 */
export function pingInterval(launch: Launch): number | null {
  const level = launch.user?.groupID ?? 999;
  if (level > Number(launch.settings.ping.auth)) {
    return null;
  }
  const ping = launch.settings.ping;
  const setting = level <= CO_ADMIN ? ping.adminRefresh : ping.everyoneRefresh;
  const interval = Number(setting);
  return Number.isFinite(interval) ? Math.max(interval, SHORTEST_INTERVAL) : null;
}

/** What the menu shows for a tab: it answers, it does not, or nothing is known. */
export function pingStatus(
  tab: MenuTab,
  results: Record<string, PingResult>,
): 'up' | 'down' | undefined {
  const address = tab.ping_url?.trim() ?? '';
  if (tab.ping !== 1 || address === '' || !Object.hasOwn(results, address)) {
    return undefined;
  }
  return results[address] === false ? 'down' : 'up';
}

/** The time of the answer as the menu shows it, for example `0.24ms`, or `null` without one. */
export function pingTime(tab: MenuTab, results: Record<string, PingResult>): string | null {
  const address = tab.ping_url?.trim() ?? '';
  if (tab.ping !== 1 || address === '' || !Object.hasOwn(results, address)) {
    return null;
  }
  const time = results[address];
  return typeof time === 'number' ? `${Number(time.toFixed(2))}ms` : null;
}
