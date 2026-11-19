import { describe, expect, it } from 'vitest';

import {
  changedSettings,
  formValues,
  GENERAL,
  hasSecret,
  LOGIN,
  MAIL,
  SECURE,
  type FieldGroup,
} from './settingsForm';

const GROUPS: FieldGroup[] = [
  {
    title: 'Test',
    fields: [
      { key: 'title', label: 'Name', kind: 'text' },
      { key: 'rememberMeDays', label: 'Days', kind: 'number', min: 1 },
      { key: 'useLogo', label: 'Logo', kind: 'switch' },
      {
        key: 'unsortedTabs',
        label: 'Place',
        kind: 'select',
        options: [
          { value: 'top', label: 'Top' },
          { value: 'bottom', label: 'Bottom' },
        ],
      },
      { key: 'organizrAPI', label: 'Key', kind: 'secret' },
    ],
  },
];

const CONFIG = {
  title: 'Home',
  rememberMeDays: '7',
  useLogo: false,
  unsortedTabs: 'top',
  organizrAPI: SECURE,
  other: 'is not in the form',
};

describe('formValues', () => {
  it('takes the stored value of each field, and starts a secret empty', () => {
    expect(formValues(GROUPS, CONFIG)).toEqual({
      title: 'Home',
      rememberMeDays: '7',
      useLogo: false,
      unsortedTabs: 'top',
      organizrAPI: '',
    });
  });

  it('gives a field that the server did not send an empty value', () => {
    expect(formValues(GROUPS, {})).toEqual({
      title: '',
      rememberMeDays: '',
      useLogo: false,
      unsortedTabs: '',
      organizrAPI: '',
    });
  });
});

describe('changedSettings', () => {
  it('is empty for a form that nobody changed', () => {
    expect(changedSettings(GROUPS, CONFIG, formValues(GROUPS, CONFIG))).toEqual({});
  });

  it('holds the fields with another value, in the form that the server takes', () => {
    const form = {
      ...formValues(GROUPS, CONFIG),
      title: 'Media',
      rememberMeDays: '30',
      useLogo: true,
      unsortedTabs: 'bottom',
    };

    expect(changedSettings(GROUPS, CONFIG, form)).toEqual({
      title: 'Media',
      rememberMeDays: '30',
      useLogo: true,
      unsortedTabs: 'bottom',
    });
  });

  it('sends a secret only when one was entered', () => {
    const form = { ...formValues(GROUPS, CONFIG), organizrAPI: 'abcdefghij0123456789' };

    expect(changedSettings(GROUPS, CONFIG, form)).toEqual({
      organizrAPI: 'abcdefghij0123456789',
    });
  });
});

describe('hasSecret', () => {
  it('is true when the server holds a value, shown or not', () => {
    expect(hasSecret({ key: SECURE }, 'key')).toBe(true);
    expect(hasSecret({ key: 'abcdefghij0123456789' }, 'key')).toBe(true);
    expect(hasSecret({ key: '' }, 'key')).toBe(false);
    expect(hasSecret({}, 'key')).toBe(false);
  });
});

describe('GENERAL and MAIL', () => {
  it('name each setting once', () => {
    for (const groups of [GENERAL, MAIL, LOGIN]) {
      const keys = groups.flatMap((group) => group.fields.map((field) => field.key));

      expect(new Set(keys).size).toBe(keys.length);
    }
  });

  it('keep the Plex token and the passwords of fallback users secret', () => {
    const fields = LOGIN.flatMap((group) => group.fields);
    const secrets = fields.filter((field) => field.kind === 'secret').map((field) => field.key);

    expect(secrets).toEqual(['plexToken', 'ombiFallbackPassword', 'overseerrFallbackPassword']);
  });

  it('keep the password of the mail server a secret, and the texts of e-mails long', () => {
    const fields = MAIL.flatMap((group) => group.fields);
    const kind = (key: string) => fields.find((field) => field.key === key)?.kind;

    expect(kind('PHPMAILER-smtpHostPassword')).toBe('secret');
    expect(kind('PHPMAILER-emailTemplateReset')).toBe('textarea');
    expect(kind('PHPMAILER-enabled')).toBe('switch');
  });
});
