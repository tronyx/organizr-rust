import { afterEach, describe, expect, it } from 'vitest';

import {
  applyAppearance,
  colourVariables,
  isColour,
  isNativeTheme,
  themeId,
  themeLabel,
} from './theme';

afterEach(() => {
  applyAppearance(undefined, {});
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

describe('a colour of the settings', () => {
  it('is a hex colour, a colour function or a name', () => {
    for (const colour of [
      '#987654',
      '#abc',
      '#11223344',
      'rgb(1, 2, 3)',
      'hsl(200deg 50% 40%)',
      'teal',
    ]) {
      expect(isColour(colour), colour).toBe(true);
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
      expect(isColour(text), text).toBe(false);
    }
  });

  it('sets the variables of its setting, and an empty or wrong one sets none', () => {
    expect(
      colourVariables({
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

describe('the look of the page', () => {
  it('takes the theme and the colours, and drops a colour that is no longer set', () => {
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
