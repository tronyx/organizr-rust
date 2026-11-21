// The look of the page: the theme, the colours of the settings "Colors & Themes", and
// the style sheet of the Admin.
//
// The page allows no style that is written into it (`style-src 'self'`). So a theme is a
// `data-theme` of the style sheet of the interface, a colour is a CSS variable that a
// script sets through the CSSOM after a check, and the style sheet of the Admin is a file
// of the server.

import { apiUrl } from './api';
import type { Appearance } from './types';

/** The daisyUI theme of each theme of PHP Organizr that this interface has rebuilt. */
const THEME_IDS: Record<string, string> = {
  Organizr: 'organizr',
  Blue: 'org-blue',
  Nord: 'org-nord',
  Dracula: 'org-dracula',
  Overseerr: 'org-overseerr',
  Onedark: 'org-onedark',
  Aquamarine: 'org-aquamarine',
  Space_Gray: 'org-space-gray',
  Plex_theme_park: 'org-plex-theme-park',
  Dark_theme_park: 'org-dark-theme-park',
  Hotline: 'org-hotline',
  Hotpink: 'org-hotpink',
  Plex: 'org-plex',
};

/** The theme of a new install. */
export const DEFAULT_THEME = 'Organizr';

/** Whether this interface has a native version of the theme of PHP Organizr. */
export function isNativeTheme(name: string): boolean {
  return Object.hasOwn(THEME_IDS, name);
}

/** The `data-theme` of a theme of PHP Organizr. A theme without a native version gets the default. */
export function themeId(name: string | undefined): string {
  return (name !== undefined && isNativeTheme(name) && THEME_IDS[name]) || 'organizr';
}

/** The name of a theme as a person reads it: `Space_Gray` is "Space Gray". */
export function themeLabel(name: string): string {
  return name.replaceAll('_', ' ');
}

const HEX = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const COLOUR_FUNCTION = /^(?:rgba?|hsla?)\([0-9a-z.,%/\s+-]*\)$/i;
const COLOUR_NAME = /^[a-z]{3,24}$/i;

/**
 * Whether a text is a colour: `#987654`, `rgb(…)`, `hsl(…)` or a name such as `teal`.
 * Nothing else reaches a CSS variable, so a setting cannot bring other CSS into the page.
 */
export function isColour(text: string): boolean {
  const value = text.trim();
  if (!HEX.test(value) && !COLOUR_FUNCTION.test(value) && !COLOUR_NAME.test(value)) {
    return false;
  }
  // A browser knows which names and forms are colours. jsdom has no `CSS.supports`.
  return typeof CSS === 'undefined' || typeof CSS.supports !== 'function'
    ? true
    : CSS.supports('color', value);
}

/** The colour settings, and the CSS variables that each one sets. */
const COLOUR_VARIABLES: [keyof Appearance, string[]][] = [
  ['headerColor', ['--org-header-bg']],
  ['headerTextColor', ['--org-header-text']],
  ['sidebarColor', ['--org-sidebar-bg']],
  ['sidebarTextColor', ['--org-sidebar-text', '--org-sidebar-active']],
  ['accentColor', ['--color-info']],
  ['accentTextColor', ['--color-info-content']],
  ['buttonColor', ['--color-primary']],
  ['buttonTextColor', ['--color-primary-content']],
];

/** Every CSS variable that a colour setting can set. */
export const COLOUR_VARIABLE_NAMES = COLOUR_VARIABLES.flatMap(([, names]) => names);

/**
 * The CSS variables of the colour settings that hold a colour. An empty setting means
 * the colour of the theme, and a setting that holds no colour is left out.
 */
export function colourVariables(appearance: Partial<Appearance>): Record<string, string> {
  const variables: Record<string, string> = {};
  for (const [key, names] of COLOUR_VARIABLES) {
    const value = appearance[key];
    if (typeof value === 'string' && value.trim() !== '' && isColour(value)) {
      for (const name of names) {
        variables[name] = value.trim();
      }
    }
  }
  return variables;
}

/** The id of the link to the style sheet of the Admin. */
const CUSTOM_CSS_ID = 'org-custom-css';

/** The texts of the style sheet that the page links now, to load it again only after a change. */
let linkedCss: string | null = null;
/** A number that makes the address of each new version of the style sheet differ. */
let cssVersion = 0;

/**
 * Links the style sheet of the Admin, after the style sheet of the interface, so that its
 * rules win. Without one, the page links nothing.
 */
function linkCustomCss(appearance: Partial<Appearance>): void {
  const text = `${appearance.customThemeCss ?? ''}\n${appearance.customCss ?? ''}`;
  const existing = document.getElementById(CUSTOM_CSS_ID);
  if (text.trim() === '') {
    existing?.remove();
    linkedCss = null;
    return;
  }
  if (existing && linkedCss === text) {
    return;
  }
  cssVersion += 1;
  const link = document.createElement('link');
  link.id = CUSTOM_CSS_ID;
  link.rel = 'stylesheet';
  link.href = `${apiUrl('appearance/custom.css')}?v=${cssVersion}`;
  existing?.remove();
  document.head.append(link);
  linkedCss = text;
}

/** Gives the page the theme, the colours and the style sheet of the settings. */
export function applyAppearance(
  theme: string | undefined,
  appearance: Partial<Appearance>,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.theme = themeId(theme);
  const variables = colourVariables(appearance);
  for (const name of COLOUR_VARIABLE_NAMES) {
    const value = variables[name];
    if (value === undefined) {
      root.style.removeProperty(name);
    } else {
      root.style.setProperty(name, value);
    }
  }
  linkCustomCss(appearance);
}
