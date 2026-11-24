// The look of the page: the theme, the colors of the settings "Colors & Themes", the
// style sheet, the scripts and the icons of the Admin.
//
// The page allows no style that is written into it (`style-src 'self'`). So a theme is a
// `data-theme` of the style sheet of the interface, a color is a CSS variable that a
// script sets through the CSSOM after a check, and the style sheet of the Admin is a file
// of the server.

import { apiUrl } from './api';
import type { Appearance, IconLink } from './types';

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

/** The names of the themes of PHP Organizr that this interface has rebuilt. */
export const NATIVE_THEMES: string[] = Object.keys(THEME_IDS);

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
const COLOR_FUNCTION = /^(?:rgba?|hsla?)\([0-9a-z.,%/\s+-]*\)$/i;
const COLOR_NAME = /^[a-z]{3,24}$/i;

/**
 * Whether a text is a color: `#987654`, `rgb(…)`, `hsl(…)` or a name such as `teal`.
 * Nothing else reaches a CSS variable, so a setting cannot bring other CSS into the page.
 */
export function isColor(text: string): boolean {
  const value = text.trim();
  if (!HEX.test(value) && !COLOR_FUNCTION.test(value) && !COLOR_NAME.test(value)) {
    return false;
  }
  // A browser knows which names and forms are colors. jsdom has no `CSS.supports`.
  return typeof CSS === 'undefined' || typeof CSS.supports !== 'function'
    ? true
    : CSS.supports('color', value);
}

/** The color settings, and the CSS variables that each one sets. */
const COLOR_VARIABLES: [keyof Appearance, string[]][] = [
  ['headerColor', ['--org-header-bg']],
  ['headerTextColor', ['--org-header-text']],
  ['sidebarColor', ['--org-sidebar-bg']],
  ['sidebarTextColor', ['--org-sidebar-text', '--org-sidebar-active']],
  ['accentColor', ['--color-info']],
  ['accentTextColor', ['--color-info-content']],
  ['buttonColor', ['--color-primary']],
  ['buttonTextColor', ['--color-primary-content']],
];

/** Every CSS variable that a color setting can set. */
export const COLOR_VARIABLE_NAMES = COLOR_VARIABLES.flatMap(([, names]) => names);

/**
 * The CSS variables of the color settings that hold a color. An empty setting means
 * the color of the theme, and a setting that holds no color is left out.
 */
export function colorVariables(appearance: Partial<Appearance>): Record<string, string> {
  const variables: Record<string, string> = {};
  for (const [key, names] of COLOR_VARIABLES) {
    const value = appearance[key];
    if (typeof value === 'string' && value.trim() !== '' && isColor(value)) {
      for (const name of names) {
        variables[name] = value.trim();
      }
    }
  }
  return variables;
}

/**
 * The Admin's colors that a sample of the install's OG theme shows (see `ThemeSwatch`): the
 * button color as its primary dot, and the accent color (daisyUI's `info` on the page)
 * as its accent dot. Only settings that hold a color count, as on the page.
 */
export function swatchColors(appearance: Partial<Appearance>): Record<string, string> {
  const variables = colorVariables(appearance);
  const colors: Record<string, string> = {};
  const primary = variables['--color-primary'];
  const accent = variables['--color-info'];
  if (primary !== undefined) {
    colors['--color-primary'] = primary;
  }
  if (accent !== undefined) {
    colors['--color-accent'] = accent;
  }
  return colors;
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
export function linkCustomCss(appearance: Partial<Appearance>): void {
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

/** Gives the page the theme, the colors and the style sheet of the settings. */
export function applyAppearance(
  theme: string | undefined,
  appearance: Partial<Appearance>,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.theme = themeId(theme);
  const variables = colorVariables(appearance);
  for (const name of COLOR_VARIABLE_NAMES) {
    const value = variables[name];
    if (value === undefined) {
      root.style.removeProperty(name);
    } else {
      root.style.setProperty(name, value);
    }
  }
  linkCustomCss(appearance);
}

/** The id of the script element of the scripts of the Admin. */
const CUSTOM_JS_ID = 'org-custom-js';

/** The texts of the scripts that the page loaded last, to load them again only after a change. */
let loadedScript: string | null = null;
/** A number that makes the address of each new version of the scripts differ. */
let scriptVersion = 0;

/**
 * Loads the scripts of the Admin from the server, as PHP Organizr runs them: on every page
 * of an install that is set up, the login page too. Only while the operator turned them
 * on, and only when there is a script. The script comes from this site, so the policy of
 * the page stays `script-src 'self'`.
 */
export function applyCustomScript(appearance: Partial<Appearance>, isSetUp: boolean): void {
  const text = `${appearance.customThemeJava ?? ''}
${appearance.customJava ?? ''}`;
  const existing = document.getElementById(CUSTOM_JS_ID);
  if (!isSetUp || appearance.customJsEnabled !== true || text.trim() === '') {
    // A script that ran keeps what it did until the page loads again.
    existing?.remove();
    loadedScript = null;
    return;
  }
  if (existing && loadedScript === text) {
    return;
  }
  scriptVersion += 1;
  const script = document.createElement('script');
  script.id = CUSTOM_JS_ID;
  script.src = `${apiUrl('appearance/custom.js')}?v=${scriptVersion}`;
  existing?.remove();
  document.body.append(script);
  loadedScript = text;
}

/** The mark of the icon links that come from the settings. */
const ICON_MARK = 'data-org-icon';

/** The icon links of the page itself, taken out while the settings name icons. */
let builtInIcons: HTMLLinkElement[] = [];

/**
 * Puts the icon links of the setting `favIcon` into the page, in place of the icon of the
 * interface. The server sent only links that it checked. Each one becomes a link element
 * of its own: no HTML of the setting reaches the page.
 */
export function applyIcons(icons: IconLink[] | undefined): void {
  for (const old of document.head.querySelectorAll(`link[${ICON_MARK}]`)) {
    old.remove();
  }
  const links = (icons ?? []).map((icon) => {
    const link = document.createElement('link');
    link.setAttribute(ICON_MARK, '');
    link.rel = icon.rel;
    link.setAttribute('href', icon.href);
    if (icon.type) {
      link.type = icon.type;
    }
    if (icon.sizes) {
      link.setAttribute('sizes', icon.sizes);
    }
    return link;
  });
  if (links.length > 0) {
    const shown = document.head.querySelectorAll<HTMLLinkElement>('link[rel~="icon"]');
    for (const link of shown) {
      link.remove();
      builtInIcons.push(link);
    }
    document.head.append(...links);
  } else {
    document.head.append(...builtInIcons);
    builtInIcons = [];
  }
}
