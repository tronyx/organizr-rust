import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';
import type { Entry, FormPage, SettingField } from '../model';
import { isField, isSecret, settingKeys } from '../values';
import expectedKeys from './expected-keys.json';
import { allPages, HOMEPAGE_ITEMS, PLUGIN_FORMS } from './index';

// The tests run in the folder `web`. The registry of the server says which settings are
// secrets: `GET /config` sends `***Secure***` for them.
const registry = readFileSync(resolve(process.cwd(), '../crates/settings/src/registry.rs'), 'utf8');

/** The British spellings that no fixed text may hold. */
const BRITISH = /\b(colour|colours|cancelled|behaviour|centre|favourite|licence)\b/i;

/** The fixed texts of an entry: everything but the labels of its choices. */
function textsOf(entry: Entry): string[] {
  const texts: (string | undefined)[] = [];
  if ('label' in entry) texts.push(entry.label);
  if ('help' in entry) texts.push(entry.help);
  if ('placeholder' in entry) texts.push(entry.placeholder);
  if ('text' in entry) texts.push(entry.text);
  if ('title' in entry) texts.push(entry.title);
  if ('body' in entry) texts.push(entry.body);
  if ('fold' in entry) texts.push(entry.fold);
  return texts.filter((text): text is string => text !== undefined);
}

function textsOfPage(page: FormPage): string[] {
  return [
    page.title,
    ...page.sections.flatMap((section) => [section.title, ...section.entries.flatMap(textsOf)]),
  ];
}

describe('the definitions of the settings forms', () => {
  it('give every setting of the old forms a field', () => {
    const keys = [...new Set(allPages().flatMap(settingKeys))].sort();
    expect(keys).toEqual(expectedKeys);
  });

  it('define no key twice on one page', () => {
    for (const page of allPages()) {
      const keys = settingKeys(page);
      expect(
        keys.filter((key, index) => keys.indexOf(key) !== index),
        page.id,
      ).toEqual([]);
    }
  });

  it('write every fixed text in American spelling', () => {
    for (const page of allPages()) {
      expect(
        textsOfPage(page).filter((text) => BRITISH.test(text)),
        page.id,
      ).toEqual([]);
    }
  });

  it('give each page and each section of a page its own id', () => {
    const ids = allPages().map((page) => page.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const page of allPages()) {
      const sections = page.sections.map((section) => section.id);
      expect(new Set(sections).size, page.id).toBe(sections.length);
    }
  });

  it('draw every setting that the server masks as a secret', () => {
    const secrets = [...registry.matchAll(/:\s*Secret\s*=\s*"([^"]+)"/g)].map(
      (match) => match[1] ?? '',
    );
    expect(secrets.length).toBeGreaterThan(0);
    const notSecret = allPages()
      .flatMap((page) => page.sections.flatMap((section) => section.entries))
      .filter((entry): entry is SettingField => isField(entry) && secrets.includes(entry.key))
      .filter((field) => !isSecret(field))
      .map((field) => field.key);
    expect(notSecret).toEqual([]);
  });

  it('draw as a secret only a setting that the server masks', () => {
    const secrets = [...registry.matchAll(/:\s*Secret\s*=\s*"([^"]+)"/g)].map(
      (match) => match[1] ?? '',
    );
    const notMasked = allPages()
      .flatMap((page) => page.sections.flatMap((section) => section.entries))
      .filter((entry): entry is SettingField => isField(entry) && isSecret(entry))
      .filter((field) => !secrets.includes(field.key))
      .map((field) => field.key);
    expect([...new Set(notMasked)]).toEqual([]);
  });

  it('hold the forty homepage items and the six plugin forms', () => {
    expect(HOMEPAGE_ITEMS).toHaveLength(40);
    expect(Object.keys(PLUGIN_FORMS).sort()).toEqual([
      'chat',
      'healthchecks',
      'invites',
      'php-mailer',
      'plexlibraries',
      'speedtest',
    ]);
  });
});
