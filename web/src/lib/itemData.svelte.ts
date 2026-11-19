// The data of a homepage item: loaded when the item shows, and again after each interval.

import { onMount } from 'svelte';

import { api, failureText } from './api';
import { session } from './session.svelte';

/** The shortest time between two loads, in milliseconds. */
export const SHORTEST_REFRESH = 5000;

/** The time between two loads when the settings name none, in milliseconds. */
export const DEFAULT_REFRESH = 60000;

/** The time between two loads of an item, from the setting with the given key. */
export function refreshTime(refresh: Record<string, unknown> | undefined, key: string): number {
  const stored = Number(refresh?.[key]);
  if (!Number.isFinite(stored) || stored <= 0) {
    return DEFAULT_REFRESH;
  }
  return Math.max(stored, SHORTEST_REFRESH);
}

/** What an item knows about its data. */
export interface ItemData<T> {
  /** The data of the last load that passed, or nothing before the first one. */
  readonly data: T | null;
  /** Why the last load failed, or nothing when it passed. */
  readonly error: string | null;
  /** Loads the data now. */
  reload(): Promise<void>;
}

/**
 * Loads the data of an item from `path` while the component that calls this is shown.
 * `refreshKey` is the key of the setting that holds the time between two loads.
 */
export function itemData<T>(path: string, refreshKey: string): ItemData<T> {
  let data = $state<T | null>(null);
  let error = $state<string | null>(null);

  async function reload() {
    try {
      data = (await api<T>('GET', path)).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  onMount(() => {
    void reload();
    const every = refreshTime(session.launch?.settings.homepage.refresh, refreshKey);
    const timer = setInterval(() => void reload(), every);
    return () => clearInterval(timer);
  });

  return {
    get data() {
      return data;
    },
    get error() {
      return error;
    },
    reload,
  };
}
