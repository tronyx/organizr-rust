import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  applyLook,
  contrastRatio,
  MODERN_THEME_COLOURS,
  MODERN_THEMES,
  modernThemeId,
  type Look,
} from './look';
import { COLOUR_VARIABLE_NAMES } from './theme';

const og = (theme: string, colourSet = 'dark'): Look => ({
  layout: 'og',
  theme,
  colourSet,
  source: 'default',
});
const modern = (theme: string, colourSet: string): Look => ({
  layout: 'modern',
  theme,
  colourSet,
  source: 'user',
});

afterEach(() => {
  applyLook(og('Organizr'), undefined, {});
});

describe('applyLook', () => {
  it('og_look_sets_layout_og_and_the_og_theme', () => {
    const root = document.createElement('html');
    applyLook(og('Nord'), undefined, {}, root);
    expect(root.dataset.layout).toBe('og');
    expect(root.dataset.theme).toBe('org-nord');
    expect(root.dataset.colourSet).toBeUndefined();
  });

  it('modern_look_sets_the_modern_theme_and_colour_set', () => {
    const root = document.createElement('html');
    applyLook(modern('organizr', 'light'), undefined, {}, root);
    expect(root.dataset.layout).toBe('modern');
    expect(root.dataset.theme).toBe('m-organizr-light');
    expect(root.dataset.colourSet).toBe('light');
  });

  it('switching_to_modern_drops_the_og_colours_and_keeps_custom_css', () => {
    const root = document.createElement('html');
    const appearance = { headerColor: '#112233', accentColor: '#445566', customCss: 'a{b:c}' };
    applyLook(og('Organizr'), undefined, appearance, root);
    expect(root.style.getPropertyValue('--org-header-bg')).toBe('#112233');
    expect(document.getElementById('org-custom-css')).not.toBeNull();

    applyLook(modern('organizr', 'dark'), undefined, appearance, root);
    for (const name of COLOUR_VARIABLE_NAMES) {
      expect(root.style.getPropertyValue(name)).toBe('');
    }
    expect(document.getElementById('org-custom-css')).not.toBeNull();
    expect(root.dataset.theme).toBe('m-organizr-dark');
  });

  it('unknown_modern_pair_uses_organizr_dark', () => {
    expect(modernThemeId('organizr', 'dark')).toBe('m-organizr-dark');
    expect(modernThemeId('organizr', 'light')).toBe('m-organizr-light');
    expect(modernThemeId('gone', 'light')).toBe('m-organizr-dark');
    expect(modernThemeId('organizr', 'sepia')).toBe('m-organizr-dark');
    expect(modernThemeId('toString', 'dark')).toBe('m-organizr-dark');
    const root = document.createElement('html');
    applyLook(modern('gone', 'x'), undefined, {}, root);
    expect(root.dataset.theme).toBe('m-organizr-dark');
  });

  it('lists_the_modern_themes', () => {
    expect(MODERN_THEMES).toEqual([
      {
        id: 'organizr',
        label: 'Organizr',
        colourSets: [
          { id: 'dark', label: 'Dark' },
          { id: 'light', label: 'Light' },
        ],
      },
    ]);
  });
});

describe('the modern colour sets', () => {
  it('modern_colour_sets_meet_aa', () => {
    expect(Object.keys(MODERN_THEME_COLOURS).sort()).toEqual([
      'm-organizr-dark',
      'm-organizr-light',
    ]);
    for (const [id, pairs] of Object.entries(MODERN_THEME_COLOURS)) {
      expect(pairs.length).toBeGreaterThan(5);
      for (const pair of pairs) {
        expect(
          contrastRatio(pair.text, pair.background),
          `${id} ${pair.name}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('app_css_defines_the_colours_of_the_table', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8');
    for (const [id, pairs] of Object.entries(MODERN_THEME_COLOURS)) {
      const start = css.indexOf(`name: '${id}';`);
      expect(start, id).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      for (const pair of pairs) {
        expect(block, `${id} ${pair.name}`).toContain(`${pair.textVariable}: ${pair.text};`);
        expect(block, `${id} ${pair.name}`).toContain(
          `${pair.backgroundVariable}: ${pair.background};`,
        );
      }
    }
  });
});
