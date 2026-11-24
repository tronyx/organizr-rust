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
  /**
   * The animated scene behind the page, for a scene theme: a key of `SCENE_LOADERS` in
   * `src/scenes`. The other themes have none.
   */
  scene?: string;
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
  {
    id: 'silo',
    label: 'Silo',
    // Silo is always dark; its sets differ in the accent only.
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
  {
    id: 'riot',
    label: 'Riot',
    scene: 'riot',
    colourSets: [
      { id: 'acid', label: 'Acid' },
      { id: 'hotwire', label: 'Hotwire' },
      { id: 'blacklight', label: 'Blacklight' },
    ],
  },
];

/** The two colour sets of most themes. */
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

/**
 * The scene that a look shows behind the page, or `null`: only a scene theme of the modern
 * layout has one, and only with a colour set of its own (else the page has the default).
 */
export function sceneOf(look: Look): string | null {
  if (look.layout !== 'modern') {
    return null;
  }
  const id = modernThemeId(look.theme, look.colourSet);
  return MODERN_THEMES.find((entry) => id.startsWith(`m-${entry.id}-`))?.scene ?? null;
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
  'm-silo-ember': [
    pair('page', '--color-base-content', '#ecedef', '--color-base-100', '#0d0e11'),
    pair('menu', '--color-base-content', '#ecedef', '--color-base-200', '#16181c'),
    pair('heading', '--color-base-content', '#ecedef', '--color-base-300', '#21242a'),
    pair('primary', '--color-primary-content', '#111215', '--color-primary', '#eceae5'),
    pair('success', '--color-success-content', '#04150d', '--color-success', '#4cc38a'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0506', '--color-error', '#f2555a'),
    pair('secondary', '--color-secondary-content', '#0d0e11', '--color-secondary', '#a3a9b5'),
    pair('accent', '--color-accent-content', '#0d0e11', '--color-accent', '#ff8f57'),
    pair('neutral', '--color-neutral-content', '#ecedef', '--color-neutral', '#08090b'),
    pair('header', '--org-header-text', '#f2f3f5', '--org-header-bg', '#121317'),
    pair('sidebar', '--org-sidebar-text', '#b6bac2', '--org-sidebar-bg', '#0a0b0d'),
    pair('sidebar active', '--org-sidebar-active', '#ff8f57', '--org-sidebar-bg', '#0a0b0d'),
  ],
  'm-silo-ice': [
    pair('page', '--color-base-content', '#ecedef', '--color-base-100', '#0d0e11'),
    pair('menu', '--color-base-content', '#ecedef', '--color-base-200', '#16181c'),
    pair('heading', '--color-base-content', '#ecedef', '--color-base-300', '#21242a'),
    pair('primary', '--color-primary-content', '#111215', '--color-primary', '#eceae5'),
    pair('success', '--color-success-content', '#04150d', '--color-success', '#4cc38a'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0506', '--color-error', '#f2555a'),
    pair('secondary', '--color-secondary-content', '#0d0e11', '--color-secondary', '#a3a9b5'),
    pair('accent', '--color-accent-content', '#0d0e11', '--color-accent', '#6cc3f7'),
    pair('neutral', '--color-neutral-content', '#ecedef', '--color-neutral', '#08090b'),
    pair('header', '--org-header-text', '#f2f3f5', '--org-header-bg', '#121317'),
    pair('sidebar', '--org-sidebar-text', '#b6bac2', '--org-sidebar-bg', '#0a0b0d'),
    pair('sidebar active', '--org-sidebar-active', '#6cc3f7', '--org-sidebar-bg', '#0a0b0d'),
  ],
  'm-silo-rose': [
    pair('page', '--color-base-content', '#ecedef', '--color-base-100', '#0d0e11'),
    pair('menu', '--color-base-content', '#ecedef', '--color-base-200', '#16181c'),
    pair('heading', '--color-base-content', '#ecedef', '--color-base-300', '#21242a'),
    pair('primary', '--color-primary-content', '#111215', '--color-primary', '#eceae5'),
    pair('success', '--color-success-content', '#04150d', '--color-success', '#4cc38a'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0506', '--color-error', '#f2555a'),
    pair('secondary', '--color-secondary-content', '#0d0e11', '--color-secondary', '#a3a9b5'),
    pair('accent', '--color-accent-content', '#0d0e11', '--color-accent', '#ff7593'),
    pair('neutral', '--color-neutral-content', '#ecedef', '--color-neutral', '#08090b'),
    pair('header', '--org-header-text', '#f2f3f5', '--org-header-bg', '#121317'),
    pair('sidebar', '--org-sidebar-text', '#b6bac2', '--org-sidebar-bg', '#0a0b0d'),
    pair('sidebar active', '--org-sidebar-active', '#ff7593', '--org-sidebar-bg', '#0a0b0d'),
  ],
  'm-nocturne-amethyst': [
    pair('page', '--color-base-content', '#ece5f6', '--color-base-100', '#120c1c'),
    pair('menu', '--color-base-content', '#ece5f6', '--color-base-200', '#1c1429'),
    pair('heading', '--color-base-content', '#ece5f6', '--color-base-300', '#2a1f3b'),
    pair('primary', '--color-primary-content', '#170b28', '--color-primary', '#b994f2'),
    pair('success', '--color-success-content', '#04150d', '--color-success', '#5fc99c'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0509', '--color-error', '#f2637d'),
    pair('secondary', '--color-secondary-content', '#1d0d24', '--color-secondary', '#d3a6e0'),
    pair('accent', '--color-accent-content', '#1a1206', '--color-accent', '#e6c27a'),
    pair('neutral', '--color-neutral-content', '#ece5f6', '--color-neutral', '#0a0710'),
    pair('header', '--org-header-text', '#f2edf9', '--org-header-bg', '#170f24'),
    pair('sidebar', '--org-sidebar-text', '#bfb3d2', '--org-sidebar-bg', '#0d0914'),
    pair('sidebar active', '--org-sidebar-active', '#c7a6f7', '--org-sidebar-bg', '#0d0914'),
  ],
  'm-nocturne-crimson': [
    pair('page', '--color-base-content', '#f2e4e7', '--color-base-100', '#140a0d'),
    pair('menu', '--color-base-content', '#f2e4e7', '--color-base-200', '#1f1015'),
    pair('heading', '--color-base-content', '#f2e4e7', '--color-base-300', '#2e1820'),
    pair('primary', '--color-primary-content', '#1c0409', '--color-primary', '#ee6f86'),
    pair('success', '--color-success-content', '#04150d', '--color-success', '#5fc99c'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1d0603', '--color-error', '#ff7a6b'),
    pair('secondary', '--color-secondary-content', '#1f0a10', '--color-secondary', '#d79aa9'),
    pair('accent', '--color-accent-content', '#1a1206', '--color-accent', '#e3bc78'),
    pair('neutral', '--color-neutral-content', '#f2e4e7', '--color-neutral', '#0b0608'),
    pair('header', '--org-header-text', '#f7eef0', '--org-header-bg', '#1a0d11'),
    pair('sidebar', '--org-sidebar-text', '#cdb5bb', '--org-sidebar-bg', '#0f080a'),
    pair('sidebar active', '--org-sidebar-active', '#f58a9c', '--org-sidebar-bg', '#0f080a'),
  ],
  'm-nocturne-moonlit': [
    pair('page', '--color-base-content', '#e6e9f6', '--color-base-100', '#0c0e1a'),
    pair('menu', '--color-base-content', '#e6e9f6', '--color-base-200', '#141728'),
    pair('heading', '--color-base-content', '#e6e9f6', '--color-base-300', '#202439'),
    pair('primary', '--color-primary-content', '#0b0f22', '--color-primary', '#a9b9f2'),
    pair('success', '--color-success-content', '#04150d', '--color-success', '#5fc99c'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1a0509', '--color-error', '#f2637d'),
    pair('secondary', '--color-secondary-content', '#100c24', '--color-secondary', '#b3a8e6'),
    pair('accent', '--color-accent-content', '#17140a', '--color-accent', '#ebdfae'),
    pair('neutral', '--color-neutral-content', '#e6e9f6', '--color-neutral', '#07080f'),
    pair('header', '--org-header-text', '#f0f2fa', '--org-header-bg', '#10132a'),
    pair('sidebar', '--org-sidebar-text', '#b4bad2', '--org-sidebar-bg', '#090b16'),
    pair('sidebar active', '--org-sidebar-active', '#bccaf7', '--org-sidebar-bg', '#090b16'),
  ],
  'm-riot-acid': [
    pair('page', '--color-base-content', '#eef3e4', '--color-base-100', '#0a0b08'),
    pair('menu', '--color-base-content', '#eef3e4', '--color-base-200', '#12140f'),
    pair('heading', '--color-base-content', '#eef3e4', '--color-base-300', '#1d2117'),
    pair('primary', '--color-primary-content', '#0c1200', '--color-primary', '#b4f224'),
    pair('success', '--color-success-content', '#03150b', '--color-success', '#4fd98a'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1d0409', '--color-error', '#ff5c7a'),
    pair('secondary', '--color-secondary-content', '#171400', '--color-secondary', '#f0e64a'),
    pair('accent', '--color-accent-content', '#101800', '--color-accent', '#c9ff5c'),
    pair('neutral', '--color-neutral-content', '#eef3e4', '--color-neutral', '#050604'),
    pair('header', '--org-header-text', '#f3f8e9', '--org-header-bg', '#0f110b'),
    pair('sidebar', '--org-sidebar-text', '#bcc6ab', '--org-sidebar-bg', '#070806'),
    pair('sidebar active', '--org-sidebar-active', '#c9ff5c', '--org-sidebar-bg', '#070806'),
  ],
  'm-riot-hotwire': [
    pair('page', '--color-base-content', '#f4ecf8', '--color-base-100', '#0d0a13'),
    pair('menu', '--color-base-content', '#f4ecf8', '--color-base-200', '#16111e'),
    pair('heading', '--color-base-content', '#f4ecf8', '--color-base-300', '#231a2e'),
    pair('primary', '--color-primary-content', '#1d0010', '--color-primary', '#ff47a8'),
    pair('success', '--color-success-content', '#03150b', '--color-success', '#4fd98a'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1d0603', '--color-error', '#ff6b5e'),
    pair('secondary', '--color-secondary-content', '#22041a', '--color-secondary', '#ff9bd2'),
    pair('accent', '--color-accent-content', '#00131d', '--color-accent', '#3fd0ff'),
    pair('neutral', '--color-neutral-content', '#f4ecf8', '--color-neutral', '#07050a'),
    pair('header', '--org-header-text', '#f8f1fb', '--org-header-bg', '#120d1a'),
    pair('sidebar', '--org-sidebar-text', '#c5b8cf', '--org-sidebar-bg', '#09070d'),
    pair('sidebar active', '--org-sidebar-active', '#3fd0ff', '--org-sidebar-bg', '#09070d'),
  ],
  'm-riot-blacklight': [
    pair('page', '--color-base-content', '#efe8fb', '--color-base-100', '#0c0817'),
    pair('menu', '--color-base-content', '#efe8fb', '--color-base-200', '#150e25'),
    pair('heading', '--color-base-content', '#efe8fb', '--color-base-300', '#221736'),
    pair('primary', '--color-primary-content', '#10031f', '--color-primary', '#b874ff'),
    pair('success', '--color-success-content', '#03150b', '--color-success', '#5fe0a0'),
    pair('warning', '--color-warning-content', '#1b1a1a', '--color-warning', '#f5b301'),
    pair('error', '--color-error-content', '#1d0409', '--color-error', '#ff6383'),
    pair('secondary', '--color-secondary-content', '#1d0426', '--color-secondary', '#ec85ff'),
    pair('accent', '--color-accent-content', '#03181a', '--color-accent', '#7af7e6'),
    pair('neutral', '--color-neutral-content', '#efe8fb', '--color-neutral', '#06040c'),
    pair('header', '--org-header-text', '#f4effc', '--org-header-bg', '#110b20'),
    pair('sidebar', '--org-sidebar-text', '#beb2d6', '--org-sidebar-bg', '#08050f'),
    pair('sidebar active', '--org-sidebar-active', '#7af7e6', '--org-sidebar-bg', '#08050f'),
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
