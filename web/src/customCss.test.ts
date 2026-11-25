// docs/custom-css.md promises that its classes and variables stay. These tests fail when
// the interface stops using a name of that page, or when the page misses a kind of item.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

// The tests run in the folder `web`.
const doc = readFileSync(resolve(process.cwd(), '../docs/custom-css.md'), 'utf8');

const SOURCES = import.meta.glob<string>('./**/*.{svelte,ts}', {
  query: '?raw',
  import: 'default',
  eager: true,
});

// Vitest empties style sheets that a test imports, so the style sheets come from the disk:
// the main one and the one of the modern layout.
const code = Object.entries(SOURCES)
  .filter(([path]) => !path.endsWith('.test.ts'))
  .map(([, text]) => text)
  .concat(readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8'))
  .concat(readFileSync(resolve(process.cwd(), 'src/modern.css'), 'utf8'))
  .join('\n');

const homepage = SOURCES['./pages/homepage/Homepage.svelte'] ?? '';

/** The kinds of items that the homepage shows, from its list `KNOWN`. */
function knownKinds(): string[] {
  const list = /const KNOWN = new Set\(\[([^\]]*)\]/.exec(homepage)?.[1] ?? '';
  return [...list.matchAll(/'([a-z]+)'/g)].map((match) => match[1] ?? '');
}

/** The kinds that the documentation lists, in its paragraph "The kinds of items". */
function documentedKinds(): string[] {
  const paragraph = doc.split('\n').find((line) => line.startsWith('The kinds of items:')) ?? '';
  return [...paragraph.matchAll(/`([a-z]+)`/g)].map((match) => match[1] ?? '');
}

describe('the names of docs/custom-css.md', () => {
  it('are classes that the interface uses', () => {
    const classes = new Set(
      [...doc.matchAll(/`\.(org-[a-z-]*[a-z])/g)].map((match) => match[1] ?? ''),
    );
    const missing = [...classes].filter((name) => {
      const kind = /^org-item-([a-z]+)$/.exec(name)?.[1];
      if (kind !== undefined && name !== 'org-item-title') {
        // The homepage writes the class of each item from its kind.
        return !knownKinds().includes(kind);
      }
      return !new RegExp(`(^|[^a-z-])${name}([^a-z-]|$)`).test(code);
    });
    expect(classes.size).toBeGreaterThan(30);
    expect(missing).toEqual([]);
  });

  it('are variables that the interface uses', () => {
    const variables = new Set([...doc.matchAll(/`(--[a-z0-9-]+)`/g)].map((match) => match[1]));
    const missing = [...variables].filter((name) => !code.includes(`${name}`));
    expect(variables.size).toBeGreaterThan(15);
    expect(missing).toEqual([]);
  });

  it('list every kind of homepage item', () => {
    expect(documentedKinds().sort()).toEqual(knownKinds().sort());
  });
});
