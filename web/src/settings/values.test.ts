import { describe, expect, it } from 'vitest';
import { SECURE } from '../lib/settingsForm';
import type { ButtonEntry, FormPage, SettingField } from './model';
import { changedSettings, initialValues, isShown, settingKeys } from './values';

function field(key: string, kind: SettingField['kind'], extra: Partial<SettingField> = {}) {
  return { key, kind, label: key, ...extra } satisfies SettingField;
}

function page(...fields: SettingField[]): FormPage {
  return {
    id: 'p',
    title: 'P',
    sections: [
      {
        id: 's',
        title: 'S',
        entries: [{ kind: 'notice', tone: 'info', body: 'Hello' }, ...fields],
      },
    ],
  };
}

describe('settingKeys', () => {
  it('lists the keys of the fields and skips the other entries', () => {
    expect(settingKeys(page(field('a', 'text'), field('b', 'switch')))).toEqual(['a', 'b']);
  });
});

describe('initialValues', () => {
  it('reads a switch as true for true, "true", 1 and "1"', () => {
    const p = page(
      field('a', 'switch'),
      field('b', 'switch'),
      field('c', 'switch'),
      field('d', 'switch'),
      field('e', 'switch'),
    );
    const values = initialValues(p, { a: true, b: 'true', c: 1, d: '1', e: 'false' });
    expect(values).toEqual({ a: true, b: true, c: true, d: true, e: false });
  });

  it('reads a stored secret as empty', () => {
    const p = page(field('a', 'secret'), field('b', 'secret-copy'), field('c', 'secret'));
    expect(initialValues(p, { a: SECURE, b: SECURE, c: 'x' })).toEqual({ a: '', b: '', c: 'x' });
  });

  it('reads the text and the array of a multi-select or tags field', () => {
    const p = page(
      field('a', 'multi-select'),
      field('b', 'tags'),
      field('c', 'tags'),
      field('d', 'multi-select'),
    );
    const values = initialValues(p, { a: 'a,b', b: ['a', 'b'], c: '', d: undefined });
    expect(values).toEqual({ a: ['a', 'b'], b: ['a', 'b'], c: [], d: [] });
  });

  it('gives an empty text for a missing key and the JSON text of an object', () => {
    const p = page(field('a', 'text'), field('b', 'text'), field('c', 'number'));
    expect(initialValues(p, { b: { x: 1 }, c: 5 })).toEqual({ a: '', b: '{"x":1}', c: '5' });
  });
});

describe('changedSettings', () => {
  const p = page(
    field('sw', 'switch'),
    field('tg', 'tags'),
    field('pw', 'secret'),
    field('tx', 'text'),
  );
  const start = { sw: false, tg: ['a'], pw: '', tx: 'x' };

  it('sends nothing when nothing changed', () => {
    expect(changedSettings(p, start, { ...start })).toEqual({});
  });

  it('sends a changed switch as a boolean', () => {
    expect(changedSettings(p, start, { ...start, sw: true })).toEqual({ sw: true });
  });

  it('sends tags as comma-separated text', () => {
    expect(changedSettings(p, start, { ...start, tg: ['a', 'b'] })).toEqual({ tg: 'a,b' });
  });

  it('does not send a secret left empty, and sends one that was typed', () => {
    expect(changedSettings(p, { ...start, pw: 'old' }, { ...start, pw: '' })).toEqual({});
    expect(changedSettings(p, start, { ...start, pw: 'new' })).toEqual({ pw: 'new' });
  });

  it('sends a changed text', () => {
    expect(changedSettings(p, start, { ...start, tx: 'y' })).toEqual({ tx: 'y' });
  });
});

describe('isShown', () => {
  it('shows a field without rules', () => {
    expect(isShown(field('a', 'text'), {}, 'personal')).toBe(true);
  });

  it('hides every sign-in field when the type is internal', () => {
    const values = { authType: 'internal', authBackend: 'plex' };
    for (const rule of ['backend', 'plex', 'emby', 'jellyfin', 'ftp', 'ldap'] as const) {
      expect(isShown(field('a', 'text', { shownWhen: rule }), values, 'personal')).toBe(false);
    }
  });

  it('shows the fields of the chosen back end and the common ones', () => {
    const values = { authType: 'both', authBackend: 'emby_connect' };
    expect(isShown(field('a', 'text', { shownWhen: 'emby' }), values, 'personal')).toBe(true);
    expect(isShown(field('a', 'text', { shownWhen: 'backend' }), values, 'personal')).toBe(true);
    expect(isShown(field('a', 'text', { shownWhen: 'plex' }), values, 'personal')).toBe(false);
  });

  it('shows a field with several back ends for each of them', () => {
    const f = field('a', 'text', { shownWhen: ['ldap', 'ftp'] });
    expect(isShown(f, { authType: 'both', authBackend: 'ftp' }, 'personal')).toBe(true);
    expect(isShown(f, { authType: 'both', authBackend: 'ldap' }, 'personal')).toBe(true);
    expect(isShown(f, { authType: 'both', authBackend: 'plex' }, 'personal')).toBe(false);
  });

  it('applies the rules of a button as those of a field', () => {
    const button = {
      kind: 'button',
      label: 'Test Login',
      action: { kind: 'login-test', path: 'test/ldap/login' },
      shownWhen: 'ldap',
    } satisfies ButtonEntry;
    expect(isShown(button, { authType: 'both', authBackend: 'ldap' }, 'personal')).toBe(true);
    expect(isShown(button, { authType: 'both', authBackend: 'plex' }, 'personal')).toBe(false);
  });

  it('shows a field only for the license it names', () => {
    const f = field('a', 'text', { license: 'personal' });
    expect(isShown(f, {}, 'personal')).toBe(true);
    expect(isShown(f, {}, 'business')).toBe(false);
  });
});
