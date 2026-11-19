// The form of the general settings: which settings it shows, and how the values of the
// form go to the server and come from it.

/** What the server sends for a secret that has a value. The value itself stays there. */
export const SECURE = '***Secure***';

interface FieldBase {
  /** The key of the setting, as `GET /config` and `PUT /config` name it. */
  key: string;
  label: string;
  /** One sentence under the field that says what the setting does. */
  help?: string;
}

export type Field =
  | (FieldBase & { kind: 'text' })
  /** A whole number. The server stores it as text, as the PHP application does. */
  | (FieldBase & { kind: 'number'; min: number })
  | (FieldBase & { kind: 'switch' })
  | (FieldBase & { kind: 'select'; options: { value: string; label: string }[] })
  /** A password or a key. `generate` adds a button that makes a random key. */
  | (FieldBase & { kind: 'secret'; generate?: boolean });

export interface FieldGroup {
  title: string;
  fields: Field[];
}

/** The general settings, in the groups of the page. */
export const GENERAL: FieldGroup[] = [
  {
    title: 'Name and look',
    fields: [
      { key: 'title', label: 'Name of this install', kind: 'text' },
      {
        key: 'description',
        label: 'Description',
        kind: 'text',
        help: 'Search engines and link previews show this text.',
      },
      {
        key: 'logo',
        label: 'Picture for the menu',
        kind: 'text',
        help: 'The address of a picture.',
      },
      { key: 'useLogo', label: 'Show the picture in the menu, not the name', kind: 'switch' },
    ],
  },
  {
    title: 'Menu',
    fields: [
      {
        key: 'unsortedTabs',
        label: 'Tabs without a heading stand',
        kind: 'select',
        options: [
          { value: 'top', label: 'Above the categories' },
          { value: 'bottom', label: 'Below the categories' },
        ],
      },
    ],
  },
  {
    title: 'Login',
    fields: [
      {
        key: 'rememberMeDays',
        label: 'Days that a login lasts with “remember me”',
        kind: 'number',
        min: 1,
      },
      {
        key: 'loginAttempts',
        label: 'Failed logins before a lock',
        kind: 'number',
        min: 1,
      },
      {
        key: 'loginLockout',
        label: 'Length of a lock, in milliseconds',
        kind: 'number',
        min: 0,
      },
      {
        key: 'localIPList',
        label: 'More addresses of the local network',
        kind: 'text',
        help: 'Addresses and networks, separated by commas. For example 100.64.0.0/10.',
      },
    ],
  },
  {
    title: 'New accounts',
    fields: [
      { key: 'hideRegistration', label: 'Hide the link to make an account', kind: 'switch' },
      {
        key: 'registrationPassword',
        label: 'Registration password',
        kind: 'secret',
        help: 'A person who knows this password can make an account.',
      },
    ],
  },
  {
    title: 'API key',
    fields: [
      {
        key: 'organizrAPI',
        label: 'API key',
        kind: 'secret',
        generate: true,
        help: 'A script that sends this key has the access of an Admin. The key has 20 characters.',
      },
    ],
  },
];

/** The value of a field in the form. */
export type FormValue = string | boolean;

/** The values of the form for the settings that the server sent. A secret starts empty. */
export function formValues(
  groups: FieldGroup[],
  config: Record<string, unknown>,
): Record<string, FormValue> {
  const values: Record<string, FormValue> = {};
  for (const field of groups.flatMap((group) => group.fields)) {
    const stored = config[field.key];
    if (field.kind === 'switch') {
      values[field.key] = stored === true;
    } else if (field.kind === 'secret') {
      values[field.key] = '';
    } else {
      values[field.key] = typeof stored === 'string' ? stored : '';
    }
  }
  return values;
}

/** Whether the server holds a value for a secret. */
export function hasSecret(config: Record<string, unknown>, key: string): boolean {
  const stored = config[key];
  return typeof stored === 'string' && stored !== '';
}

/**
 * The settings to send: the fields whose value in the form differs from the stored one.
 * A secret is sent only when the person entered one.
 */
export function changedSettings(
  groups: FieldGroup[],
  config: Record<string, unknown>,
  form: Record<string, FormValue>,
): Record<string, FormValue> {
  const stored = formValues(groups, config);
  const changed: Record<string, FormValue> = {};
  for (const field of groups.flatMap((group) => group.fields)) {
    const value = form[field.key];
    if (value !== undefined && value !== stored[field.key]) {
      changed[field.key] = value;
    }
  }
  return changed;
}
