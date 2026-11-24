import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  applyLook,
  contrastRatio,
  MODERN_THEME_COLOURS,
  MODERN_THEMES,
  modernThemeId,
  sceneOf,
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
    applyLook(modern('organizr', 'dark'), undefined, {}, root);
    expect(root.dataset.layout).toBe('modern');
    expect(root.dataset.theme).toBe('m-organizr-dark');
    expect(root.dataset.colourSet).toBe('dark');
  });

  it('the_light_colour_set_is_offered', () => {
    const root = document.createElement('html');
    applyLook(modern('organizr', 'light'), undefined, {}, root);
    expect(root.dataset.theme).toBe('m-organizr-light');
    expect(root.dataset.colourSet).toBe('light');
  });

  it('a_fresh_modern_look_links_the_custom_css', () => {
    document.getElementById('org-custom-css')?.remove();
    const root = document.createElement('html');
    applyLook(modern('organizr', 'dark'), undefined, { customCss: 'a{}' }, root);
    expect(document.querySelectorAll('#org-custom-css')).toHaveLength(1);
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
    const sets = [
      { id: 'dark', label: 'Dark' },
      { id: 'light', label: 'Light' },
    ];
    expect(MODERN_THEMES).toEqual([
      { id: 'organizr', label: 'Organizr', colourSets: sets },
      { id: 'plex', label: 'Plex', colourSets: sets },
      { id: 'jellyfin', label: 'Jellyfin', colourSets: sets },
      { id: 'emby', label: 'Emby', colourSets: sets },
      {
        id: 'silo',
        label: 'Silo',
        colourSets: [
          { id: 'ember', label: 'Ember' },
          { id: 'ice', label: 'Ice' },
          { id: 'rose', label: 'Rose' },
        ],
      },
      {
        id: 'nocturne',
        label: 'Nocturne',
        scene: 'nocturne',
        colourSets: [
          { id: 'amethyst', label: 'Amethyst' },
          { id: 'crimson', label: 'Crimson' },
          { id: 'moonlit', label: 'Moonlit' },
        ],
      },
    ]);
  });

  it('the_brand_themes_have_their_own_ids', () => {
    for (const theme of ['plex', 'jellyfin', 'emby']) {
      for (const set of ['dark', 'light']) {
        expect(modernThemeId(theme, set)).toBe(`m-${theme}-${set}`);
        const root = document.createElement('html');
        applyLook(modern(theme, set), undefined, {}, root);
        expect(root.dataset.theme).toBe(`m-${theme}-${set}`);
        expect(root.dataset.colourSet).toBe(set);
      }
      expect(modernThemeId(theme, 'purple')).toBe('m-organizr-dark');
    }
  });

  it('silo_has_three_accent_sets_and_no_dark_set', () => {
    for (const set of ['ember', 'ice', 'rose']) {
      expect(modernThemeId('silo', set)).toBe(`m-silo-${set}`);
      const root = document.createElement('html');
      applyLook(modern('silo', set), undefined, {}, root);
      expect(root.dataset.theme).toBe(`m-silo-${set}`);
      // Every set of Silo is dark.
      expect(root.dataset.colourSet).toBe('dark');
    }
    // `dark` and `light` are not sets of Silo: they give the default, as any unknown set.
    expect(modernThemeId('silo', 'dark')).toBe('m-organizr-dark');
    expect(modernThemeId('silo', 'light')).toBe('m-organizr-dark');
  });

  it('nocturne_has_three_dark_colour_sets', () => {
    for (const set of ['amethyst', 'crimson', 'moonlit']) {
      expect(modernThemeId('nocturne', set)).toBe(`m-nocturne-${set}`);
      const root = document.createElement('html');
      applyLook(modern('nocturne', set), undefined, {}, root);
      expect(root.dataset.theme).toBe(`m-nocturne-${set}`);
      expect(root.dataset.colourSet).toBe('dark');
    }
    expect(modernThemeId('nocturne', 'dark')).toBe('m-organizr-dark');
  });
});

describe('sceneOf', () => {
  it('only_a_scene_theme_on_the_modern_layout_has_a_scene', () => {
    expect(sceneOf(modern('nocturne', 'amethyst'))).toBe('nocturne');
    expect(sceneOf(modern('nocturne', 'moonlit'))).toBe('nocturne');
    for (const theme of ['organizr', 'plex', 'jellyfin', 'emby']) {
      expect(sceneOf(modern(theme, 'dark')), theme).toBeNull();
    }
    expect(sceneOf(modern('silo', 'ember'))).toBeNull();
    // A set the theme does not have gives the default theme, which has no scene.
    expect(sceneOf(modern('nocturne', 'dark'))).toBeNull();
    expect(sceneOf({ ...modern('nocturne', 'amethyst'), layout: 'og' })).toBeNull();
    expect(sceneOf(og('Organizr'))).toBeNull();
  });
});

describe('the modern colour sets', () => {
  it('modern_colour_sets_meet_aa', () => {
    expect(Object.keys(MODERN_THEME_COLOURS).sort()).toEqual([
      'm-emby-dark',
      'm-emby-light',
      'm-jellyfin-dark',
      'm-jellyfin-light',
      'm-nocturne-amethyst',
      'm-nocturne-crimson',
      'm-nocturne-moonlit',
      'm-organizr-dark',
      'm-organizr-light',
      'm-plex-dark',
      'm-plex-light',
      'm-silo-ember',
      'm-silo-ice',
      'm-silo-rose',
    ]);
    const names = (MODERN_THEME_COLOURS['m-organizr-dark'] ?? []).map((pair) => pair.name);
    expect(names).toHaveLength(13);
    for (const [id, pairs] of Object.entries(MODERN_THEME_COLOURS)) {
      expect(
        pairs.map((pair) => pair.name),
        id,
      ).toEqual(names);
      for (const pair of pairs) {
        expect(
          contrastRatio(pair.text, pair.background),
          `${id} ${pair.name}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('muted_text_meets_aa_on_each_offered_background', () => {
    // `--color-muted-foreground` is the text colour at the opacity of the theme's
    // `--org-muted-text` (65% where a theme sets none) over the background.
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8');
    expect(css.replace(/\s+/g, ' ')).toContain(
      '--color-muted-foreground: color-mix( in oklab, var(--color-base-content) var(--org-muted-text, 65%), transparent );',
    );
    const strength = (id: string) => {
      const start = css.indexOf(`name: '${id}';`);
      const block = css.slice(start, css.indexOf('}', start));
      const found = /--org-muted-text: (\d+)%;/.exec(block);
      return found ? Number(found[1]) / 100 : 0.65;
    };
    const channels = (hex: string) =>
      [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
    const mixed = (text: string, background: string, part: number) =>
      `#${channels(text)
        .map((value, i) =>
          Math.round(part * value + (1 - part) * (channels(background)[i] ?? 0))
            .toString(16)
            .padStart(2, '0'),
        )
        .join('')}`;
    for (const id of MODERN_THEMES.flatMap((theme) =>
      theme.colourSets.map((set) => modernThemeId(theme.id, set.id)),
    )) {
      const pairs = MODERN_THEME_COLOURS[id] ?? [];
      const text = pairs.find((pair) => pair.name === 'page')?.text ?? '';
      for (const name of ['page', 'menu', 'heading']) {
        const background = pairs.find((pair) => pair.name === name)?.background ?? '';
        expect(
          contrastRatio(mixed(text, background, strength(id)), background),
          `${id} muted on ${name}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('the_active_menu_item_meets_aa_on_its_highlight', () => {
    // The open entry of the side menu has the active colour on a base-300 highlight.
    for (const [id, pairs] of Object.entries(MODERN_THEME_COLOURS)) {
      const active = pairs.find((pair) => pair.name === 'sidebar active')?.text ?? '';
      const highlight = pairs.find((pair) => pair.name === 'heading')?.background ?? '';
      expect(contrastRatio(active, highlight), id).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('the_text_of_the_jellyfin_gradient_meets_aa_on_both_ends', () => {
    // The primary buttons of Jellyfin run from the secondary colour to the primary one,
    // with the primary text colour on both.
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8');
    for (const id of ['m-jellyfin-dark', 'm-jellyfin-light']) {
      const start = css.indexOf(`name: '${id}';`);
      expect(start, id).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      const value = (name: string) =>
        new RegExp(`--color-${name}: (#[0-9a-f]{6});`).exec(block)?.[1] ?? '';
      for (const end of ['primary', 'secondary']) {
        expect(
          contrastRatio(value('primary-content'), value(end)),
          `${id} on ${end}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('silo_has_off_white_buttons_with_dark_text', () => {
    for (const id of ['m-silo-ember', 'm-silo-ice', 'm-silo-rose']) {
      const pairs = MODERN_THEME_COLOURS[id] ?? [];
      const primary = pairs.find((pair) => pair.name === 'primary');
      const page = pairs.find((pair) => pair.name === 'page');
      // Off-white: nearly as light as white, far lighter than the page.
      expect(contrastRatio(primary?.background ?? '', '#ffffff'), id).toBeLessThan(1.25);
      expect(contrastRatio(primary?.background ?? '', page?.background ?? ''), id).toBeGreaterThan(
        12,
      );
      expect(contrastRatio(primary?.text ?? '', '#000000'), id).toBeLessThan(1.25);
    }
  });

  it('the_silo_accent_reads_as_text_and_as_a_focus_ring', () => {
    // Silo shows its accent for links, the help texts, the open menu entry and the focus
    // ring. As text it needs 4.5:1 on every surface; as a ring 3:1 on the card behind it.
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8');
    for (const id of ['m-silo-ember', 'm-silo-ice', 'm-silo-rose']) {
      const start = css.indexOf(`name: '${id}';`);
      expect(start, id).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      const value = (name: string) =>
        new RegExp(`--${name}: (#[0-9a-f]{6});`).exec(block)?.[1] ?? '-';
      const accent = value('color-accent');
      for (const surface of ['color-base-100', 'color-base-200', 'color-base-300']) {
        expect(contrastRatio(accent, value(surface)), `${id} on ${surface}`).toBeGreaterThanOrEqual(
          4.5,
        );
      }
      // The open entry of the side menu is in the accent too.
      expect(value('org-sidebar-active'), id).toBe(accent);
    }
  });

  it('nocturne_accents_and_buttons_read_on_every_surface', () => {
    // Nocturne marks links, the open settings entry and the focus ring with its accent,
    // and its primary buttons sit on the cards: both need 4.5:1 on every surface.
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8');
    for (const set of ['amethyst', 'crimson', 'moonlit']) {
      const id = `m-nocturne-${set}`;
      const start = css.indexOf(`name: '${id}';`);
      expect(start, id).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      const value = (name: string) =>
        new RegExp(`--${name}: (#[0-9a-f]{6});`).exec(block)?.[1] ?? '-';
      for (const colour of ['color-accent', 'color-primary']) {
        for (const surface of ['color-base-100', 'color-base-200', 'color-base-300']) {
          expect(
            contrastRatio(value(colour), value(surface)),
            `${id} ${colour} on ${surface}`,
          ).toBeGreaterThanOrEqual(4.5);
        }
      }
      expect(block, id).toContain('color-scheme: dark;');
    }
  });

  it('the_contrast_ratio_has_known_values', () => {
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 5);
    expect(contrastRatio('#ffffff', '#ffffff')).toBeCloseTo(1, 5);
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
