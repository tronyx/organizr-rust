// The values of a settings form: what the fields hold when the page opens, which settings
// the Admin changed, and which fields show.

import { SECURE } from '../lib/settingsForm';
import type { Entry, FormPage, FormValue, SettingField } from './model';

/** Whether an entry is a field that holds a setting. */
function isField(entry: Entry): entry is SettingField {
  return 'key' in entry;
}

/** Every field of the page, in order. */
function fieldsOf(page: FormPage): SettingField[] {
  return page.sections.flatMap((section) => section.entries.filter(isField));
}

/** Whether the field is a secret: the server only says that one is stored. */
function isSecret(field: SettingField): boolean {
  return field.kind === 'secret' || field.kind === 'secret-copy';
}

/** Whether the field holds a list of values. */
function isList(field: SettingField): boolean {
  return field.kind === 'multi-select' || field.kind === 'tags';
}

/** The keys of the settings that the page holds, in order. */
export function settingKeys(page: FormPage): string[] {
  return fieldsOf(page).map((field) => field.key);
}

/** The value of a field in the form, from the value that `GET /config` sent. */
function valueOf(field: SettingField, value: unknown): FormValue {
  if (field.kind === 'switch') {
    return value === true || value === 'true' || value === 1 || value === '1';
  }
  if (isSecret(field)) {
    // A stored secret is shown as empty; the field says that a value is stored.
    return value === SECURE ? '' : String(value ?? '');
  }
  if (isList(field)) {
    if (Array.isArray(value)) {
      return value.map(String);
    }
    const text = String(value ?? '');
    return text === '' ? [] : text.split(',');
  }
  if (value === null || value === undefined) {
    return '';
  }
  return typeof value === 'object' ? JSON.stringify(value) : String(value);
}

/** The values of the form when the page opens, by the key of each setting. */
export function initialValues(
  page: FormPage,
  config: Record<string, unknown>,
): Record<string, FormValue> {
  const values: Record<string, FormValue> = {};
  for (const field of fieldsOf(page)) {
    values[field.key] = valueOf(field, config[field.key]);
  }
  return values;
}

/**
 * The settings that the Admin changed, as `PUT /config` takes them. A secret that is left
 * empty keeps its stored value.
 */
export function changedSettings(
  page: FormPage,
  start: Record<string, FormValue>,
  now: Record<string, FormValue>,
): Record<string, unknown> {
  const changed: Record<string, unknown> = {};
  for (const field of fieldsOf(page)) {
    const before = start[field.key];
    const after = now[field.key];
    if (after === undefined || JSON.stringify(before) === JSON.stringify(after)) {
      continue;
    }
    if (isSecret(field) && after === '') {
      continue;
    }
    changed[field.key] = Array.isArray(after) ? after.join(',') : after;
  }
  return changed;
}

/** The sign-in back ends by the setting `authBackend`, as PHP's `changeAuth()` maps them. */
const BACKEND_RULES: Record<string, string> = {
  plex: 'plex',
  emby_local: 'emby',
  emby_connect: 'emby',
  emby_all: 'emby',
  jellyfin: 'jellyfin',
  ftp: 'ftp',
  ldap: 'ldap',
};

/**
 * Whether a field shows, with the values as they stand in the form and the stored license
 * (`personal` or `business`).
 */
export function isShown(
  field: SettingField,
  values: Record<string, FormValue>,
  license: string,
): boolean {
  if (field.license !== undefined && !field.license.includes(license)) {
    return false;
  }
  if (field.shownWhen === undefined) {
    return true;
  }
  if (values.authType === 'internal') {
    return false;
  }
  if (field.shownWhen === 'backend') {
    return true;
  }
  return BACKEND_RULES[String(values.authBackend ?? '')] === field.shownWhen;
}
