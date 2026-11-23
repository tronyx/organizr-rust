// The look of the page: the layout (the original one, "og", or the modern one) and the
// theme of each. The server sends the look that applies to the caller in `GET /launch`.

import { applyAppearance, COLOUR_VARIABLE_NAMES, linkCustomCss } from './theme';
import type { Appearance } from './types';

/** The layout and the theme that the page shows. */
export interface Look {
  layout: 'og' | 'modern';
  /** On OG the name of a theme of PHP Organizr, on modern the id of a modern theme. */
  theme: string;
  /** On OG the style (`dark` or `light`), on modern a colour set of the theme. */
  colourSet: string;
  /** Whether the person chose it (`user`) or the Admin default applies. */
  source: 'user' | 'default';
}

export interface ModernTheme {
  id: string;
  label: string;
  colourSets: { id: string; label: string }[];
}

/**
 * The themes of the modern layout. Later versions add entries. The light colour set of
 * `organizr` is defined in `app.css` but not offered: the inner pages keep OG styling that
 * is unreadable on it. Phase 2 adds it back here and in the server's list.
 */
export const MODERN_THEMES: ModernTheme[] = [
  {
    id: 'organizr',
    label: 'Organizr',
    colourSets: [{ id: 'dark', label: 'Dark' }],
  },
];

/** The `data-theme` that a modern theme and colour set have. Unknown values give the default. */
export function modernThemeId(theme: string, colourSet: string): string {
  const known = MODERN_THEMES.find(
    (entry) => entry.id === theme && entry.colourSets.some((set) => set.id === colourSet),
  );
  return known ? `m-${known.id}-${colourSet}` : 'm-organizr-dark';
}

/** A text colour on a background, both as CSS variables of the theme in `app.css`. */
export interface ColourPair {
  name: string;
  textVariable: string;
  text: string;
  backgroundVariable: string;
  background: string;
}

function pair(
  name: string,
  textVariable: string,
  text: string,
  backgroundVariable: string,
  background: string,
): ColourPair {
  return { name, textVariable, text, backgroundVariable, background };
}

/**
 * The text and background colours of each modern theme. `app.css` defines the same values
 * (a test compares them), and a test checks that each pair meets WCAG AA (4.5:1).
 */
export const MODERN_THEME_COLOURS: Record<string, ColourPair[]> = {
  'm-organizr-dark': [
    pair('page', '--color-base-content', '#dce2ec', '--color-base-100', '#1a212b'),
    pair('menu', '--color-base-content', '#dce2ec', '--color-base-200', '#141a22'),
    pair('heading', '--color-base-content', '#dce2ec', '--color-base-300', '#232c39'),
    pair('primary', '--color-primary-content', '#06141d', '--color-primary', '#2cabe3'),
    pair('success', '--color-success-content', '#04201a', '--color-success', '#00c292'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#fec107'),
    pair('error', '--color-error-content', '#1a0509', '--color-error', '#f33155'),
    pair('secondary', '--color-secondary-content', '#0d1030', '--color-secondary', '#9aa4e6'),
    pair('accent', '--color-accent-content', '#1b1a1a', '--color-accent', '#e5a00d'),
    pair('neutral', '--color-neutral-content', '#dce2ec', '--color-neutral', '#0f141a'),
    pair('header', '--org-header-text', '#f1f5f9', '--org-header-bg', '#1f2733'),
    pair('sidebar', '--org-sidebar-text', '#b4c0d0', '--org-sidebar-bg', '#141a22'),
    pair('sidebar active', '--org-sidebar-active', '#5cc6f2', '--org-sidebar-bg', '#141a22'),
  ],
  'm-organizr-light': [
    pair('page', '--color-base-content', '#1f2a37', '--color-base-100', '#f3f6fa'),
    pair('menu', '--color-base-content', '#1f2a37', '--color-base-200', '#ffffff'),
    pair('heading', '--color-base-content', '#1f2a37', '--color-base-300', '#e4e9f0'),
    pair('primary', '--color-primary-content', '#ffffff', '--color-primary', '#0a74a6'),
    pair('success', '--color-success-content', '#ffffff', '--color-success', '#0a7d5e'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#ffffff', '--color-error', '#c4203f'),
    pair('secondary', '--color-secondary-content', '#ffffff', '--color-secondary', '#4c58b8'),
    pair('accent', '--color-accent-content', '#ffffff', '--color-accent', '#9a6700'),
    pair('neutral', '--color-neutral-content', '#ffffff', '--color-neutral', '#2b3544'),
    pair('header', '--org-header-text', '#1f2a37', '--org-header-bg', '#ffffff'),
    pair('sidebar', '--org-sidebar-text', '#475569', '--org-sidebar-bg', '#ffffff'),
    pair('sidebar active', '--org-sidebar-active', '#0a6a98', '--org-sidebar-bg', '#ffffff'),
  ],
};

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((start) => {
    const value = Number.parseInt(hex.slice(start, start + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (channels[0] ?? 0) + 0.7152 * (channels[1] ?? 0) + 0.0722 * (channels[2] ?? 0);
}

/** The WCAG contrast ratio of two colours written as `#rrggbb`. */
export function contrastRatio(first: string, second: string): number {
  const [light, dark] = [luminance(first), luminance(second)].sort((a, b) => b - a);
  return ((light ?? 0) + 0.05) / ((dark ?? 0) + 0.05);
}

/**
 * Gives the page its layout and theme. `theme` is the theme that an older server sends
 * outside `look`; it serves when the look names none.
 */
export function applyLook(
  look: Look,
  theme: string | undefined,
  appearance: Partial<Appearance>,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.layout = look.layout;
  if (look.layout === 'modern') {
    root.dataset.theme = modernThemeId(look.theme, look.colourSet);
    root.dataset.colourSet = root.dataset.theme.endsWith('-light') ? 'light' : 'dark';
    // The colours of "Colors & Themes" belong to the OG layout; the theme sets its own.
    for (const name of COLOUR_VARIABLE_NAMES) {
      root.style.removeProperty(name);
    }
    linkCustomCss(appearance);
    return;
  }
  delete root.dataset.colourSet;
  applyAppearance(look.theme || theme, appearance, root);
}
