import { afterEach, describe, expect, it, vi } from 'vitest';

import { setLanguage, startLanguages, translate } from './i18n.svelte';

const GERMAN = {
  Username: 'Benutzername',
  'Health Checks': 'Gesundheitschecks',
  Save: 'Speichern',
};

function stubPack() {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (address: string) =>
      address.endsWith('langpack/de.json')
        ? new Response(JSON.stringify(GERMAN))
        : new Response('', { status: 404 }),
    ),
  );
}

/** Lets the watcher of the page see the changes. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe('i18n', () => {
  afterEach(async () => {
    await setLanguage('en');
    document.body.innerHTML = '';
    document.cookie = 'organizrLanguage=; path=/; max-age=0';
    vi.unstubAllGlobals();
  });

  it('translates the texts and the attributes of the page, and goes back to English', async () => {
    stubPack();
    document.body.innerHTML = `
      <h2> Health Checks : </h2>
      <input placeholder="Username">
      <p translate="no">Save</p>
      <button>Save</button>`;
    await startLanguages();
    await setLanguage('de');

    expect(document.querySelector('h2')?.textContent).toBe(' Gesundheitschecks : ');
    expect(document.querySelector('input')?.getAttribute('placeholder')).toBe('Benutzername');
    // A text of the data is left as it is.
    expect(document.querySelector('p')?.textContent).toBe('Save');
    expect(document.querySelector('button')?.textContent).toBe('Speichern');
    expect(document.documentElement.lang).toBe('de');
    expect(document.cookie).toContain('organizrLanguage=de');

    await setLanguage('en');
    expect(document.querySelector('h2')?.textContent).toBe(' Health Checks : ');
    expect(document.querySelector('button')?.textContent).toBe('Save');
  });

  it('translates what the interface adds or changes later', async () => {
    stubPack();
    await startLanguages();
    await setLanguage('de');

    const button = document.createElement('button');
    button.textContent = 'Save';
    document.body.append(button);
    await settle();
    expect(button.textContent).toBe('Speichern');

    // The interface writes a new English text: it is the new original.
    (button.firstChild as Text).nodeValue = 'Username';
    await settle();
    expect(button.textContent).toBe('Benutzername');
  });

  it('leaves a text without a translation as it is', () => {
    expect(translate('Something new')).toBe('Something new');
  });
});
