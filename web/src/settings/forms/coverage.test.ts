import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';
import type { Entry, FormPage, SettingField } from '../model';
import { isField, isSecret, settingKeys } from '../values';
import expectedKeys from './expected-keys.json';
import expectedKinds from './expected-kinds.json';
import { allPages, HOMEPAGE_ITEMS, PLUGIN_FORMS } from './index';
import { PHP_MAILER_PLUGIN } from './plugins/phpMailer';

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

/** Every field of every page. */
function allFields(): SettingField[] {
  return allPages()
    .flatMap((page) => page.sections.flatMap((section) => section.entries))
    .filter(isField);
}

/** The field of a key on a page, which the test needs to find. */
function fieldOf(page: FormPage, key: string): SettingField {
  const found = page.sections
    .flatMap((section) => section.entries)
    .find((entry): entry is SettingField => isField(entry) && entry.key === key);
  if (!found) {
    throw new Error(`${page.id} has no field ${key}`);
  }
  return found;
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

  it('hold the forty-one homepage items and the six plugin forms', () => {
    expect(HOMEPAGE_ITEMS).toHaveLength(41);
    expect(Object.keys(PLUGIN_FORMS).sort()).toEqual([
      'chat',
      'healthchecks',
      'invites',
      'php-mailer',
      'plexlibraries',
      'speedtest',
    ]);
  });

  it('keep the kind of every key on every page', () => {
    const found = Object.fromEntries(
      allPages().map((page) => [
        page.id,
        Object.fromEntries(
          page.sections
            .flatMap((section) => section.entries)
            .filter(isField)
            .map((field) => [field.key, field.kind]),
        ),
      ]),
    );
    expect(found).toEqual(expectedKinds);
  });

  it('hide the Plex machine ID on every page that holds it', () => {
    const fields = allFields().filter((field) => field.key === 'plexID');
    expect(fields).toHaveLength(4);
    for (const field of fields) {
      expect(field.label).toBe('Plex Machine ID');
      expect(field.kind).toBe('text');
      expect(field.masked).toBe(true);
    }
  });

  it('offer Open in New Window first on the three Target Tab selects', () => {
    const items = Object.fromEntries(HOMEPAGE_ITEMS.map((item) => [item.id, item.page]));
    for (const [item, key] of [
      ['lidarr', 'lidarrFrameTarget'],
      ['radarr', 'radarrFrameTarget'],
      ['sonarr', 'sonarrFrameTarget'],
    ] as const) {
      const page = items[item];
      expect(page, item).toBeDefined();
      const field = fieldOf(page as FormPage, key);
      expect(field.label, key).toBe('Target Tab');
      expect(field.source, key).toBe('frame-tabs');
      expect(field.choices?.[0], key).toEqual({ label: 'Open in New Window', value: '' });
    }
  });

  it('title the four custom mail templates by their stored names', () => {
    for (const number of ['One', 'Two', 'Three', 'Four']) {
      const name = `PHPMAILER-emailTemplateCustom-include-${number}Name`;
      for (const key of [name, `PHPMAILER-emailTemplateCustom-include-${number}Subject`]) {
        expect(fieldOf(PHP_MAILER_PLUGIN, key).foldTitleKey, key).toBe(name);
      }
      expect(
        fieldOf(PHP_MAILER_PLUGIN, `PHPMAILER-emailTemplateCustom-include-${number}`).foldTitleKey,
      ).toBe(name);
    }
  });

  it('keep the corrected spellings of the old forms', () => {
    const texts = allPages().flatMap((page) => [
      ...textsOfPage(page),
      ...page.sections.flatMap((section) =>
        section.entries.flatMap((entry) =>
          isField(entry) ? (entry.choices ?? []).map((choice) => choice.label) : [],
        ),
      ),
    ]);
    expect(texts.filter((text) => /\b(Tueday|adress)\b/i.test(text))).toEqual([]);
    const days = allFields().filter((field) =>
      (field.choices ?? []).some((choice) => choice.value === '2' && choice.label === 'Tuesday'),
    );
    expect(days.length).toBeGreaterThanOrEqual(7);
    expect(texts).toContain('Emby server address');
  });
});
