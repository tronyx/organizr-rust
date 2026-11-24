import { describe, expect, it } from 'vitest';
import type { Entry, FormPage } from '../model';
import { settingKeys } from '../values';
import expectedKeys from './expected-keys.json';
import { allPages, HOMEPAGE_ITEMS, PLUGIN_FORMS } from './index';

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

  it('hold the forty homepage items and the five plugin forms', () => {
    expect(HOMEPAGE_ITEMS).toHaveLength(40);
    expect(Object.keys(PLUGIN_FORMS).sort()).toEqual([
      'healthchecks',
      'invites',
      'php-mailer',
      'plexlibraries',
      'speedtest',
    ]);
  });
});
