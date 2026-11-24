// The common rules of the icon fonts reach every icon, also one whose markup names only its
// family (`oi-themify ti-trash`) and not the class `oi`. Without them an icon in an `<i>`
// element is drawn in italics, slanted by the browser.
import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

// The tests run in the folder `web`.
const folder = resolve(process.cwd(), 'src/icons');
const base = readFileSync(resolve(folder, 'base.css'), 'utf8').replaceAll(/\/\*[\s\S]*?\*\//g, '');

/** The class of each icon family, from the style sheets of the families. */
function families(): string[] {
  return readdirSync(folder)
    .filter((name) => name.endsWith('.css') && name !== 'base.css')
    .flatMap((name) => {
      const text = readFileSync(resolve(folder, name), 'utf8');
      return [...text.matchAll(/^\.(oi-[a-z]+) \{/gm)].map((match) => match[1] ?? '');
    });
}

describe('the common rules of the icons', () => {
  it('every_icon_family_is_upright', () => {
    const found = families();
    expect(found.length).toBeGreaterThanOrEqual(5);
    const rule = /([^{}]*)\{[^}]*font-style:\s*normal/.exec(base);
    const selectors = (rule?.[1] ?? '').split(',').map((part) => part.trim());
    expect(selectors).toContain('.oi');
    for (const family of found) {
      expect(selectors, family).toContain(`.${family}`);
    }
  });
});
