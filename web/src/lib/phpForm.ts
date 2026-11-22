// The forms of the settings as PHP Organizr builds them: groups of fields, each field with
// its type, label, help text and value. The server sends them from `GET /settings/...`
// with the groups, labels and order of PHP (D-96). This file reads them and works out
// what changed.

import { SECURE } from './settingsForm';

/** A choice of a list: what PHP shows, and the value that it stores. */
export interface PhpOption {
  name: string;
  value: string | number | boolean;
  disabled?: boolean;
}

/** A field of a form of PHP, as the server sends it. */
export interface PhpField {
  type: string;
  label?: string;
  name?: string | null;
  value?: unknown;
  help?: string;
  placeholder?: string;
  class?: string;
  options?: PhpOption[] | string;
  /** The options of PHP's select2, as the text of a JavaScript object. */
  settings?: string;
  /** The text of a button. */
  text?: string;
  /** The picture of a button, as classes of PHP. */
  icon?: string;
  /** How many of the twelve columns of a row the field takes, when it is not six. */
  override?: number;
  /** A code editor: `css`, `html` or `javascript`. */
  mode?: string;
  /** A notice: its colour, its title and its text. */
  notice?: string;
  title?: string;
  body?: string;
  /** The text of the type `about`. */
  about?: string;
  /** The short label of a field inside a part of an accordion. */
  smallLabel?: string;
  disabled?: boolean;
}

/** The groups of a form, by their names, in the order of PHP. */
export type PhpGroups = Record<string, PhpField[]>;

/** The value of a field as the form holds it. */
export type PhpValue = string | boolean | string[];

/** The types whose value is a secret: the server sends only that one is stored. */
const SECRET_TYPES = new Set(['password', 'password-alt', 'password-alt-copy']);

/** The types that are no setting: they hold text, a picture or a button. */
const NO_SETTING = new Set([
  'blank',
  'button',
  'notice',
  'about',
  'socks',
  'cronfile',
  'html',
  'accordion',
  // The services of the health checks: the page has its own editor for them.
  'arrayMultiple',
]);

/** Whether a field stores a setting, and so has a value in the form. */
export function holdsSetting(field: PhpField): field is PhpField & { name: string } {
  return !NO_SETTING.has(field.type) && typeof field.name === 'string' && field.name !== '';
}

/** Whether the field is a secret. */
export function isSecret(field: PhpField): boolean {
  return SECRET_TYPES.has(field.type);
}

/** The choices of a list. A list that PHP fills later is a text, and has none here. */
export function optionsOf(field: PhpField): PhpOption[] {
  return Array.isArray(field.options) ? field.options : [];
}

/** Whether a list takes several values: PHP's `select2` with the class `select2-multiple`. */
export function isMultiple(field: PhpField): boolean {
  return field.type === 'select2';
}

/** Whether a list takes values that the Admin types: PHP's select2 with `tags: true`. */
export function isTags(field: PhpField): boolean {
  return isMultiple(field) && /\btags:\s*true\b/.test(field.settings ?? '');
}

/** Whether the values of a list are secrets, which PHP shows as dots (`password-alt`). */
export function hidesValues(field: PhpField): boolean {
  return /password-alt/.test(field.settings ?? '');
}

/** The value of a field in the form, from the value that the server sent. */
export function formValue(field: PhpField): PhpValue {
  const value = field.value;
  if (field.type === 'switch') {
    return value === true || value === 'true' || value === 1 || value === '1';
  }
  if (isSecret(field)) {
    // A stored secret is shown as empty; the field says that a value is stored.
    return value === SECURE ? '' : String(value ?? '');
  }
  if (isMultiple(field)) {
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

/** A part of an accordion: its heading and its fields. */
export interface AccordionPart {
  id: string;
  header: string;
  body: PhpField[];
}

/** The parts of an accordion, for example the templates of the e-mails. */
export function accordionParts(field: PhpField): AccordionPart[] {
  return field.type === 'accordion' && Array.isArray(field.options)
    ? (field.options as unknown as AccordionPart[])
    : [];
}

/** Every field of the groups that holds a setting, also those inside an accordion. */
export function settingFields(groups: PhpGroups): (PhpField & { name: string })[] {
  return Object.values(groups)
    .flat()
    .flatMap((field) => [field, ...accordionParts(field).flatMap((part) => part.body)])
    .filter(holdsSetting);
}

/** The values of the form, by the key of each setting. */
export function formValues(groups: PhpGroups): Record<string, PhpValue> {
  const values: Record<string, PhpValue> = {};
  for (const field of settingFields(groups)) {
    values[field.name] = formValue(field);
  }
  return values;
}

/** The value that `PUT /config` gets for a value of the form. */
function sentValue(field: PhpField, value: PhpValue): unknown {
  if (Array.isArray(value)) {
    return value.join(',');
  }
  return value;
}

/**
 * The settings that the Admin changed, as `PUT /config` takes them. A secret that is left
 * empty keeps its stored value.
 */
export function changedValues(
  groups: PhpGroups,
  start: Record<string, PhpValue>,
  now: Record<string, PhpValue>,
): Record<string, unknown> {
  const changed: Record<string, unknown> = {};
  for (const field of settingFields(groups)) {
    const before = start[field.name];
    const after = now[field.name];
    if (after === undefined || JSON.stringify(before) === JSON.stringify(after)) {
      continue;
    }
    if (isSecret(field) && after === '') {
      continue;
    }
    changed[field.name] = sentValue(field, after);
  }
  return changed;
}

/** Whether a secret field has a stored value, which the server only names. */
export function hasStoredSecret(field: PhpField): boolean {
  return isSecret(field) && field.value === SECURE;
}

/**
 * Whether a field shows, with the values as they stand in the form. PHP hides the fields
 * of the back ends of the login that are not chosen (`changeAuth()` of its scripts).
 */
export function isShown(field: PhpField, form: Record<string, PhpValue>): boolean {
  const classes = (field.class ?? '').split(/\s+/);
  if (!classes.includes('switchAuth')) {
    return true;
  }
  if (form.authType === 'internal') {
    return false;
  }
  if (classes.includes('backendAuth')) {
    return true;
  }
  const backend = String(form.authBackend ?? '');
  const shown: Record<string, string> = {
    plex: 'plexAuth',
    emby_local: 'embyAuth',
    emby_connect: 'embyAuth',
    emby_all: 'embyAuth',
    jellyfin: 'jellyfinAuth',
    ftp: 'ftpAuth',
    ldap: 'ldapAuth',
  };
  const wanted = shown[backend];
  return wanted !== undefined && classes.includes(wanted);
}

/** What a button of a form does, from its name and its label, as PHP's scripts do it. */
export type PhpAction =
  /** A test of a service with its stored settings: `POST test/<name>`. */
  | { kind: 'test'; path: string; method: 'POST' | 'GET' }
  /** A test of a login with a name and a password that the Admin types. */
  | { kind: 'login-test'; path: string }
  /** A new random key for the API key of Organizr. */
  | { kind: 'new-key' }
  /** The colours go back to those of the theme. */
  | { kind: 'reset-colours' }
  /** A page that opens in a new window. */
  | { kind: 'open'; url: string }
  /** A lookup whose answer the Admin reads. */
  | { kind: 'lookup'; path: string }
  /** The token of a Plex account, from the login of Plex in a small window. */
  | { kind: 'plex-token' }
  /** The Plex servers that the account of the token owns, to choose the one of Organizr. */
  | { kind: 'plex-machine' }
  /** A button that this application has no use for. */
  | { kind: 'none'; why: string };

/** The tests of services, by the name of their button. */
const SERVICE_TESTS = new Set([
  'adguard',
  'deluge',
  'emby',
  'embyLiveTVTracker',
  'jackett',
  'jdownloader',
  'jellyfin',
  'jellystat',
  'ldap',
  'lidarr',
  'nzbget',
  'ombi',
  'overseerr',
  'pihole',
  'plex',
  'prowlarr',
  'qbittorrent',
  'radarr',
  'rtorrent',
  'sabnzbd',
  'sickrage',
  'slack-logs',
  'sonarr',
  'tautulli',
  'transmission',
  'unifi',
  'utorrent',
]);

/** What a button does. */
export function actionOf(field: PhpField): PhpAction {
  const name = field.name ?? '';
  const label = field.label ?? '';
  const text = field.text ?? '';
  if (name.startsWith('testOIDC')) {
    return { kind: 'test', path: `oidc/${name.slice(8).toLowerCase()}/test`, method: 'GET' };
  }
  if (SERVICE_TESTS.has(name)) {
    return { kind: 'test', path: `test/${name}`, method: 'POST' };
  }
  if (label === 'Send Test') {
    return { kind: 'test', path: 'plugins/php-mailer/email/test', method: 'GET' };
  }
  if (label === 'Test Login') {
    return { kind: 'login-test', path: 'test/ldap/login' };
  }
  if (label === 'Generate New API Key') {
    return { kind: 'new-key' };
  }
  if (label === 'Reset Colors') {
    return { kind: 'reset-colours' };
  }
  if (label === 'Get Plex Token') {
    return { kind: 'plex-token' };
  }
  if (label === 'Get Plex Machine') {
    return { kind: 'plex-machine' };
  }
  if (label.startsWith('Grab Unifi Site')) {
    return { kind: 'lookup', path: 'test/unifi/site' };
  }
  if (text === 'Need Help With Coordinates?') {
    return { kind: 'open', url: 'https://www.latlong.net/' };
  }
  if (text === 'Connect Account') {
    return { kind: 'open', url: 'api/v2/oauth/trakt' };
  }
  if (name === 'force-install-branch') {
    return { kind: 'none', why: 'This application runs as a container image: pull a new image.' };
  }
  return { kind: 'none', why: 'This button is not available yet.' };
}
