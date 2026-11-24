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
 * The themes of the modern layout, as in the server's list. Later versions add entries.
 */
export const MODERN_THEMES: ModernTheme[] = [
  { id: 'organizr', label: 'Organizr', colourSets: darkAndLight() },
  { id: 'plex', label: 'Plex', colourSets: darkAndLight() },
  { id: 'jellyfin', label: 'Jellyfin', colourSets: darkAndLight() },
  { id: 'emby', label: 'Emby', colourSets: darkAndLight() },
];

/** The two colour sets that each theme has so far. */
function darkAndLight(): ModernTheme['colourSets'] {
  return [
    { id: 'dark', label: 'Dark' },
    { id: 'light', label: 'Light' },
  ];
}

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
  'm-plex-dark': [
    pair('page', '--color-base-content', '#e6e6e6', '--color-base-100', '#1f1f1f'),
    pair('menu', '--color-base-content', '#e6e6e6', '--color-base-200', '#191919'),
    pair('heading', '--color-base-content', '#e6e6e6', '--color-base-300', '#2b2b2b'),
    pair('primary', '--color-primary-content', '#1a1406', '--color-primary', '#e5a00d'),
    pair('success', '--color-success-content', '#0c1a08', '--color-success', '#6abf4b'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0506', '--color-error', '#e5484d'),
    pair('secondary', '--color-secondary-content', '#1a1006', '--color-secondary', '#cc7b19'),
    pair('accent', '--color-accent-content', '#1a1406', '--color-accent', '#f2c94c'),
    pair('neutral', '--color-neutral-content', '#e6e6e6', '--color-neutral', '#121212'),
    pair('header', '--org-header-text', '#f2f2f2', '--org-header-bg', '#262626'),
    pair('sidebar', '--org-sidebar-text', '#b8b8b8', '--org-sidebar-bg', '#191919'),
    pair('sidebar active', '--org-sidebar-active', '#e5a00d', '--org-sidebar-bg', '#191919'),
  ],
  'm-plex-light': [
    pair('page', '--color-base-content', '#2a2620', '--color-base-100', '#f7f4ee'),
    pair('menu', '--color-base-content', '#2a2620', '--color-base-200', '#ffffff'),
    pair('heading', '--color-base-content', '#2a2620', '--color-base-300', '#ebe6dc'),
    pair('primary', '--color-primary-content', '#ffffff', '--color-primary', '#946300'),
    pair('success', '--color-success-content', '#ffffff', '--color-success', '#2f7d2a'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#ffffff', '--color-error', '#c4203f'),
    pair('secondary', '--color-secondary-content', '#ffffff', '--color-secondary', '#8a4f0b'),
    pair('accent', '--color-accent-content', '#ffffff', '--color-accent', '#7a5a00'),
    pair('neutral', '--color-neutral-content', '#ffffff', '--color-neutral', '#2b2b2b'),
    pair('header', '--org-header-text', '#2a2620', '--org-header-bg', '#ffffff'),
    pair('sidebar', '--org-sidebar-text', '#57504a', '--org-sidebar-bg', '#ffffff'),
    pair('sidebar active', '--org-sidebar-active', '#8a5c00', '--org-sidebar-bg', '#ffffff'),
  ],
  'm-jellyfin-dark': [
    pair('page', '--color-base-content', '#e3e8f0', '--color-base-100', '#101827'),
    pair('menu', '--color-base-content', '#e3e8f0', '--color-base-200', '#0b111c'),
    pair('heading', '--color-base-content', '#e3e8f0', '--color-base-300', '#1c2638'),
    pair('primary', '--color-primary-content', '#03101a', '--color-primary', '#00a4dc'),
    pair('success', '--color-success-content', '#04150f', '--color-success', '#3fb68b'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0509', '--color-error', '#f0506e'),
    pair('secondary', '--color-secondary-content', '#03101a', '--color-secondary', '#b06acb'),
    pair('accent', '--color-accent-content', '#1a0a22', '--color-accent', '#e0a3f5'),
    pair('neutral', '--color-neutral-content', '#e3e8f0', '--color-neutral', '#0a0f18'),
    pair('header', '--org-header-text', '#f1f5f9', '--org-header-bg', '#141e30'),
    pair('sidebar', '--org-sidebar-text', '#b3bdd0', '--org-sidebar-bg', '#0b111c'),
    pair('sidebar active', '--org-sidebar-active', '#38bdf0', '--org-sidebar-bg', '#0b111c'),
  ],
  'm-jellyfin-light': [
    pair('page', '--color-base-content', '#1a2233', '--color-base-100', '#f2f5fa'),
    pair('menu', '--color-base-content', '#1a2233', '--color-base-200', '#ffffff'),
    pair('heading', '--color-base-content', '#1a2233', '--color-base-300', '#e2e8f2'),
    pair('primary', '--color-primary-content', '#ffffff', '--color-primary', '#00709b'),
    pair('success', '--color-success-content', '#ffffff', '--color-success', '#0a7d5e'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#ffffff', '--color-error', '#c4203f'),
    pair('secondary', '--color-secondary-content', '#ffffff', '--color-secondary', '#7d3c98'),
    pair('accent', '--color-accent-content', '#ffffff', '--color-accent', '#6b3685'),
    pair('neutral', '--color-neutral-content', '#ffffff', '--color-neutral', '#1c2638'),
    pair('header', '--org-header-text', '#1a2233', '--org-header-bg', '#ffffff'),
    pair('sidebar', '--org-sidebar-text', '#4a5568', '--org-sidebar-bg', '#ffffff'),
    pair('sidebar active', '--org-sidebar-active', '#006a96', '--org-sidebar-bg', '#ffffff'),
  ],
  'm-emby-dark': [
    pair('page', '--color-base-content', '#e4e4e4', '--color-base-100', '#101010'),
    pair('menu', '--color-base-content', '#e4e4e4', '--color-base-200', '#0a0a0a'),
    pair('heading', '--color-base-content', '#e4e4e4', '--color-base-300', '#1e1e1e'),
    pair('primary', '--color-primary-content', '#061205', '--color-primary', '#52b54b'),
    pair('success', '--color-success-content', '#061205', '--color-success', '#52b54b'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0506', '--color-error', '#e5484d'),
    pair('secondary', '--color-secondary-content', '#04120f', '--color-secondary', '#4cb3a8'),
    pair('accent', '--color-accent-content', '#0f1a05', '--color-accent', '#a6d96a'),
    pair('neutral', '--color-neutral-content', '#e4e4e4', '--color-neutral', '#181818'),
    pair('header', '--org-header-text', '#f0f0f0', '--org-header-bg', '#1a1a1a'),
    pair('sidebar', '--org-sidebar-text', '#b5b5b5', '--org-sidebar-bg', '#0a0a0a'),
    pair('sidebar active', '--org-sidebar-active', '#6cc565', '--org-sidebar-bg', '#0a0a0a'),
  ],
  'm-emby-light': [
    pair('page', '--color-base-content', '#1c241c', '--color-base-100', '#f4f6f4'),
    pair('menu', '--color-base-content', '#1c241c', '--color-base-200', '#ffffff'),
    pair('heading', '--color-base-content', '#1c241c', '--color-base-300', '#e4e9e4'),
    pair('primary', '--color-primary-content', '#ffffff', '--color-primary', '#2e7d32'),
    pair('success', '--color-success-content', '#ffffff', '--color-success', '#2e7d32'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#ffffff', '--color-error', '#c4203f'),
    pair('secondary', '--color-secondary-content', '#ffffff', '--color-secondary', '#1e6f68'),
    pair('accent', '--color-accent-content', '#ffffff', '--color-accent', '#4a6f1a'),
    pair('neutral', '--color-neutral-content', '#ffffff', '--color-neutral', '#263026'),
    pair('header', '--org-header-text', '#1c241c', '--org-header-bg', '#ffffff'),
    pair('sidebar', '--org-sidebar-text', '#4a564a', '--org-sidebar-bg', '#ffffff'),
    pair('sidebar active', '--org-sidebar-active', '#246b28', '--org-sidebar-bg', '#ffffff'),
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
