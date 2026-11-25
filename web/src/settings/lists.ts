// The choices of the lists of the settings forms: the lists that come from the server
// (groups, users, frame tabs, themes, plugin lists) and the lists that come from the
// values of the form itself.

import { api } from '../lib/api';
import type { Account, EditorData, Group, Tab } from '../lib/types';
import type { Choice, FormValue, ListSource, SettingField } from './model';

/** A tab that shows in a frame, with the addresses that the source `frame-tabs` matches. */
export interface FrameTab {
  choice: Choice;
  url: string | null;
  urlLocal: string | null;
}

/** The frame tabs of a caller, and the names of all tabs by their ids. */
export interface LoadedTabs {
  frameTabs: FrameTab[];
  tabNames: Record<string, string>;
}

/**
 * The tabs that are on, show in a frame and are for a caller of the given level, as PHP's
 * `getIframeTabs()` lists them, with the names of all tabs.
 */
export async function loadTabs(level: number): Promise<LoadedTabs> {
  const { data } = await api<Omit<EditorData, 'users'>>('GET', 'tabs');
  return {
    frameTabs: data.tabs
      .filter((tab: Tab) => tab.enabled === 1 && tab.type === 1 && tab.group_id >= level)
      .map((tab: Tab) => ({
        choice: { label: tab.name, value: String(tab.id) },
        url: tab.url,
        urlLocal: tab.url_local,
      })),
    tabNames: Object.fromEntries(data.tabs.map((tab: Tab) => [String(tab.id), tab.name])),
  };
}

/** The routes of the plugin lists, which answer with their choices. */
const PLUGIN_LISTS = {
  'plex-libraries': 'plugins/plexlibraries/libraries',
  'nextcloud-groups': 'plugins/invites/nextcloud/groups',
  'komga-libraries': 'plugins/invites/komga/libraries',
} as const;

/**
 * The choices of a list that comes from the server. The source `setting-values` comes from
 * the form: it gives no choices here.
 */
export async function loadList(source: ListSource, level = 999): Promise<Choice[]> {
  switch (source) {
    case 'groups': {
      const { data } = await api<Group[]>('GET', 'groups');
      return data.map((group) => ({ label: group.group, value: String(group.group_id) }));
    }
    case 'users': {
      const { data } = await api<Account[]>('GET', 'users');
      return data.map((user) => ({ label: user.username, value: String(user.id) }));
    }
    case 'themes': {
      const { data } = await api<Record<string, unknown>>('GET', 'themes');
      return Object.keys(data).map((name) => ({ label: name, value: name }));
    }
    case 'frame-tabs':
      return (await loadTabs(level)).frameTabs.map((tab) => tab.choice);
    case 'plex-libraries':
    case 'nextcloud-groups':
    case 'komga-libraries': {
      const { data } = await api<Choice[]>('GET', PLUGIN_LISTS[source]);
      return data;
    }
    case 'setting-values':
      return [];
  }
}

/**
 * The lists that a page loaded, by their source; the frame tabs with their addresses; the
 * names of all tabs by their ids; and the sources that could not be loaded.
 */
export interface LoadedLists {
  lists: Partial<Record<ListSource, Choice[]>>;
  frameTabs: FrameTab[];
  tabNames: Record<string, string>;
  failed: ListSource[];
}

/**
 * Loads each list that the fields need once, for a caller of the given level. A list that
 * fails to load is named in `failed`, and its fields keep only their fixed choices.
 */
export async function loadLists(fields: SettingField[], level: number): Promise<LoadedLists> {
  const sources = [
    ...new Set(fields.map((field) => field.source).filter((source) => source !== undefined)),
  ];
  const loaded: LoadedLists = { lists: {}, frameTabs: [], tabNames: {}, failed: [] };
  await Promise.all(
    sources.map(async (source) => {
      try {
        if (source === 'frame-tabs') {
          ({ frameTabs: loaded.frameTabs, tabNames: loaded.tabNames } = await loadTabs(level));
        } else {
          loaded.lists[source] = await loadList(source, level);
        }
      } catch {
        loaded.failed.push(source);
      }
    }),
  );
  return loaded;
}

/** The parts of a list of values: a list, or a text with commas, as PHP's `makeOptionsFromValues()`. */
function partsOf(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.map(String).filter((part) => part !== '');
  }
  if (value === null || value === undefined || value === false) {
    return [];
  }
  const text = value === true ? '1' : String(value);
  return text.split(',').filter((part) => part !== '');
}

/**
 * The choices of a field: its fixed choices first, then those of its source. `valueOf`
 * gives the value of another setting, as it stands in the form.
 */
export function choicesOf(
  field: SettingField,
  loaded: LoadedLists,
  valueOf: (key: string) => unknown,
): Choice[] {
  const all = sourcedChoices(field, loaded, valueOf);
  return all.filter(
    (choice, index) => all.findIndex((one) => one.value === choice.value) === index,
  );
}

/** The fixed choices of a field, then those of its source, repeats and all. */
function sourcedChoices(
  field: SettingField,
  loaded: LoadedLists,
  valueOf: (key: string) => unknown,
): Choice[] {
  const fixed = field.choices ?? [];
  const source = field.source;
  if (source === undefined) {
    return fixed;
  }
  if (source === 'setting-values') {
    const values = partsOf(field.sourceArg === undefined ? '' : valueOf(field.sourceArg));
    return [...fixed, ...values.map((value) => ({ label: value, value }))];
  }
  if (source === 'frame-tabs') {
    const address = field.sourceArg === undefined ? '' : String(valueOf(field.sourceArg) ?? '');
    const tabs = loaded.frameTabs.filter(
      (tab) => address === '' || tab.url === address || tab.urlLocal === address,
    );
    return [...fixed, ...tabs.map((tab) => tab.choice)];
  }
  return [...fixed, ...(loaded.lists[source] ?? [])];
}

/**
 * The choices with each value of the field added, once, when it is none of them, as an old
 * or hand-edited value: the list shows it, and a Save leaves it as it is. An empty value
 * adds nothing: the list shows its first choice, as PHP does. `names` gives the label of a
 * value that has one, as the name of a tab; another value shows as it is.
 */
export function withValue(
  choices: Choice[],
  value: FormValue,
  names: Record<string, string> = {},
): Choice[] {
  const values = Array.isArray(value) ? value : [String(value)];
  const missing = [...new Set(values)].filter(
    (one) => one !== '' && !choices.some((choice) => choice.value === one),
  );
  return [...choices, ...missing.map((one) => ({ label: names[one] ?? one, value: one }))];
}
