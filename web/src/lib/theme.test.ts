import { afterEach, describe, expect, it } from 'vitest';

import {
  applyAppearance,
  applyCustomScript,
  applyIcons,
  colorVariables,
  isColor,
  isNativeTheme,
  swatchColors,
  themeId,
  themeLabel,
} from './theme';

afterEach(() => {
  applyAppearance(undefined, {});
  applyCustomScript({}, true);
  applyIcons([]);
});

describe('the themes', () => {
  it('have a native version for the two of PHP and the eleven of the marketplace', () => {
    for (const name of [
      'Organizr',
      'Blue',
      'Nord',
      'Dracula',
      'Overseerr',
      'Onedark',
      'Aquamarine',
      'Space_Gray',
      'Plex_theme_park',
      'Dark_theme_park',
      'Hotline',
      'Hotpink',
      'Plex',
    ]) {
      expect(isNativeTheme(name), name).toBe(true);
    }
    expect(themeId('Organizr')).toBe('organizr');
    expect(themeId('Space_Gray')).toBe('org-space-gray');
    expect(themeLabel('Plex_theme_park')).toBe('Plex theme park');
  });

  it('fall back to the default for a theme without a native version', () => {
    expect(isNativeTheme('Other')).toBe(false);
    expect(isNativeTheme('constructor')).toBe(false);
    expect(themeId('Other')).toBe('organizr');
    expect(themeId(undefined)).toBe('organizr');
  });
});

describe('a color of the settings', () => {
  it('is a hex color, a color function or a name', () => {
    for (const color of [
      '#987654',
      '#abc',
      '#11223344',
      'rgb(1, 2, 3)',
      'hsl(200deg 50% 40%)',
      'teal',
    ]) {
      expect(isColor(color), color).toBe(true);
    }
  });

  it('is never other CSS', () => {
    for (const text of [
      'red; background: url(https://example.com/x)',
      'url(https://example.com/x)',
      '#12345',
      'rgb(1,2,3)) } body { color: red',
      'var(--color-info)',
      'expression(alert(1))',
      '',
    ]) {
      expect(isColor(text), text).toBe(false);
    }
  });

  it('sets the variables of its setting, and an empty or wrong one sets none', () => {
    expect(
      colorVariables({
        headerColor: '#112233',
        sidebarTextColor: ' teal ',
        accentColor: '',
        buttonColor: 'red; color: blue',
      }),
    ).toEqual({
      '--org-header-bg': '#112233',
      '--org-sidebar-text': 'teal',
      '--org-sidebar-active': 'teal',
    });
  });
});

describe('the colors of the sample of the theme of the install', () => {
  it('take the button and accent colors, and only those that are colors', () => {
    expect(
      swatchColors({
        headerColor: '#112233',
        buttonColor: ' #ff0000 ',
        accentColor: '#00ff00',
        accentTextColor: '#000000',
      }),
    ).toEqual({ '--color-primary': '#ff0000', '--color-accent': '#00ff00' });
    expect(swatchColors({ buttonColor: 'red; color: blue', accentColor: '' })).toEqual({});
    expect(swatchColors({})).toEqual({});
  });
});

describe('the look of the page', () => {
  it('takes the theme and the colors, and drops a color that is no longer set', () => {
    const root = document.documentElement;
    applyAppearance('Nord', { headerColor: '#112233', buttonTextColor: '#ffffff' });

    expect(root.dataset.theme).toBe('org-nord');
    expect(root.style.getPropertyValue('--org-header-bg')).toBe('#112233');
    expect(root.style.getPropertyValue('--color-primary-content')).toBe('#ffffff');

    applyAppearance('Nord', { headerColor: '' });
    expect(root.style.getPropertyValue('--org-header-bg')).toBe('');
    expect(root.style.getPropertyValue('--color-primary-content')).toBe('');
  });

  it('links the style sheet of the Admin only when there is one, and again after a change', () => {
    const link = () => document.getElementById('org-custom-css') as HTMLLinkElement | null;

    applyAppearance('Organizr', { customCss: '' });
    expect(link()).toBeNull();

    applyAppearance('Organizr', { customCss: 'body { color: red; }' });
    const first = link()?.href;
    expect(first).toMatch(/\/api\/v2\/appearance\/custom\.css\?v=\d+$/);
    expect(document.head.lastElementChild).toBe(link());

    applyAppearance('Organizr', { customCss: 'body { color: red; }' });
    expect(link()?.href).toBe(first);

    applyAppearance('Organizr', { customCss: 'body { color: blue; }' });
    expect(link()?.href).not.toBe(first);

    applyAppearance('Organizr', { customCss: '', customThemeCss: '' });
    expect(link()).toBeNull();
  });
});

describe('the scripts of the Admin', () => {
  const script = () => document.getElementById('org-custom-js') as HTMLScriptElement | null;
  const on = { customJsEnabled: true, customJava: 'window.x = 1;' };

  it('load only while the operator turned them on and there is a script', () => {
    applyCustomScript({ customJsEnabled: false, customJava: 'window.x = 1;' }, true);
    expect(script()).toBeNull();
    applyCustomScript({ customJsEnabled: true, customJava: ' ', customThemeJava: '' }, true);
    expect(script()).toBeNull();
    // The set-up wizard runs no script, as in PHP.
    applyCustomScript(on, false);
    expect(script()).toBeNull();

    applyCustomScript(on, true);
    expect(script()?.src).toMatch(/\/api\/v2\/appearance\/custom\.js\?v=\d+$/);
    expect(document.querySelectorAll('script#org-custom-js')).toHaveLength(1);
  });

  it('load again after a change, and are dropped when the switch goes off', () => {
    applyCustomScript(on, true);
    const first = script()?.src;
    applyCustomScript(on, true);
    expect(script()?.src).toBe(first);
    applyCustomScript({ ...on, customThemeJava: 'window.y = 2;' }, true);
    expect(script()?.src).not.toBe(first);
    expect(document.querySelectorAll('script#org-custom-js')).toHaveLength(1);

    applyCustomScript({ ...on, customJsEnabled: false }, true);
    expect(script()).toBeNull();
  });
});

describe('the icons of the page', () => {
  it('take the place of the icon of the interface, and give it back', () => {
    const builtIn = document.createElement('link');
    builtIn.rel = 'icon';
    builtIn.href = './favicon.svg';
    document.head.append(builtIn);
    const icons = () =>
      [
        ...document.head.querySelectorAll<HTMLLinkElement>(
          'link[rel~="icon"], link[rel="apple-touch-icon"]',
        ),
      ].map((link) => [link.rel, link.getAttribute('href'), link.type, link.getAttribute('sizes')]);

    applyIcons([
      { rel: 'icon', href: 'data/favicon/favicon-32x32.png', type: 'image/png', sizes: '32x32' },
      { rel: 'apple-touch-icon', href: 'https://cdn.example.com/touch.png' },
    ]);
    expect(icons()).toEqual([
      ['icon', 'data/favicon/favicon-32x32.png', 'image/png', '32x32'],
      ['apple-touch-icon', 'https://cdn.example.com/touch.png', '', null],
    ]);

    applyIcons([]);
    expect(icons()).toEqual([['icon', './favicon.svg', '', null]]);
    builtIn.remove();
  });
});
