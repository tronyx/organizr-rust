// The section "Style & Themes" of the Appearance settings: PHP's group "Colors & Themes",
// with the default layout and the theme of each layout at its top, and one Save.
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import AppearanceSettings from './AppearanceSettings.svelte';

/** The stored look, with a color of the OG layout and PHP's `style`. */
const CONFIG = {
  layout: 'og',
  modernTheme: 'organizr',
  modernColorSet: 'dark',
  theme: 'Organizr',
  style: 'dark',
  headerColor: '',
};

function launchOf(groupID: number) {
  return {
    data: {
      user: { username: 'zed', groupID, loggedin: true, locked: 0 },
      tabs: [],
      categories: [],
      plugins: {},
      theme: 'Organizr',
      appearance: { title: 'Organizr' },
      settings: { misc: {} },
      status: { status: 'ok' },
    },
  };
}

/** Opens the page on the section, as the Admin (0) or a Co-Admin (1). */
async function open(
  config: Record<string, unknown> = CONFIG,
  groupID = 0,
  answers: Parameters<typeof stubApi>[0] = {},
) {
  const sent = stubApi({
    'GET launch': launchOf(groupID),
    'GET config': { data: { ...CONFIG, ...config } },
    // The themes that are installed, as `GET themes` gives them.
    'GET themes': { data: { Organizr: {}, Nord: {} } },
    'PUT config': {},
    ...answers,
  });
  await session.load();
  const ondone = vi.fn();
  render(AppearanceSettings, { ondone });
  await fireEvent.click(await screen.findByRole('tab', { name: 'Style & Themes' }));
  const panel = screen.getByRole('tabpanel', { name: 'Style & Themes settings' });
  await within(panel).findByRole('combobox', { name: 'Default layout' });
  const save = async () => {
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(ondone).toHaveBeenCalled());
  };
  const puts = () => sent.filter((request) => request.method === 'PUT');
  return { sent, panel, save, puts };
}

describe('the group Style & Themes', () => {
  it('loads the settings once for the form and the look', async () => {
    const { sent } = await open();

    expect(
      sent.filter((request) => request.method === 'GET' && request.path === 'config'),
    ).toHaveLength(1);
  });

  it('replaces_the_layout_tab_and_keeps_its_place', async () => {
    await open();

    // The section keeps the place of PHP's "Colors & Themes": between its neighbors.
    const tabs = screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    const at = (name: string) => tabs.indexOf(name);
    expect(at('Login page')).toBeGreaterThan(-1);
    expect(at('Style & Themes')).toBe(at('Login page') + 1);
    expect(at('Notifications and favicon')).toBe(at('Style & Themes') + 1);
    expect(screen.queryByRole('tab', { name: 'Layout' })).toBeNull();
    expect(screen.queryByRole('tab', { name: 'Colors & Themes' })).toBeNull();
  });

  it('shows_the_default_layout_then_the_colors', async () => {
    const { panel } = await open();

    const layout = within(panel).getByRole('combobox', { name: 'Default layout' });
    const heading = within(panel).getByRole('heading', {
      name: 'Colors of the OG Organizr layout',
    });
    const reset = within(panel).getByRole('button', { name: 'Reset' });
    const color = within(panel).getByLabelText('Nav Bar Color', { selector: 'input' });
    const follows = (first: Element, second: Element) =>
      (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(within(panel).getAllByRole('combobox')[0]).toBe(layout);
    expect(follows(layout, heading)).toBe(true);
    expect(follows(heading, reset)).toBe(true);
    expect(follows(reset, color)).toBe(true);
    expect(panel).toHaveTextContent('apply to the OG Organizr layout only');
  });

  it('og_shows_theme_with_the_installed_themes_once_and_no_style', async () => {
    const { panel } = await open();

    const theme = within(panel).getByRole('combobox', { name: 'Theme' });
    expect(theme).toHaveValue('Organizr');
    expect(
      within(theme)
        .getAllByRole('option')
        .map((option) => [option.textContent, (option as HTMLOptionElement).value]),
    ).toEqual([
      ['Organizr', 'Organizr'],
      ['Nord', 'Nord'],
    ]);
    // The field Theme of the definition is not shown a second time. PHP's Style has no
    // effect (D-92), so the page has no field for it.
    expect(within(panel).getAllByLabelText('Theme', { exact: true })).toHaveLength(1);
    expect(within(panel).queryByLabelText('Style', { exact: true })).toBeNull();
    expect(within(panel).queryByRole('combobox', { name: 'Modern theme' })).toBeNull();
    expect(within(panel).queryByRole('combobox', { name: 'Modern color set' })).toBeNull();
    expect(panel.querySelector('.org-swatch')).toHaveAttribute('data-theme', 'organizr');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('a_stored_theme_that_is_not_installed_still_shows_and_is_no_change', async () => {
    const { panel } = await open({ theme: 'Plexy' });

    const theme = within(panel).getByRole('combobox', { name: 'Theme' });
    expect(theme).toHaveValue('Plexy');
    expect(within(theme).getByRole('option', { name: 'Nord' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('a_failed_list_of_themes_leaves_the_stored_theme_with_a_note', async () => {
    const { panel, save, puts } = await open({}, 0, {
      'GET themes': { status: 500, message: 'The themes are away' },
    });

    const theme = within(panel).getByRole('combobox', { name: 'Theme' });
    expect(
      within(theme)
        .getAllByRole('option')
        .map((option) => option.textContent),
    ).toEqual(['Organizr']);
    expect(theme.closest('.org-form-group')).toHaveTextContent('The list could not be loaded.');
    // The rest of the look still loads and saves.
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Default layout' }), {
      target: { value: 'modern' },
    });
    await save();
    expect(puts().map((request) => request.body)).toEqual([{ layout: 'modern' }]);
  });

  it('modern_shows_the_modern_selects_and_switching_back_keeps_the_values', async () => {
    const { panel } = await open();

    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    const layout = within(panel).getByRole('combobox', { name: 'Default layout' });
    expect(
      within(layout)
        .getAllByRole('option')
        .map((option) => [option.textContent, (option as HTMLOptionElement).value]),
    ).toEqual([
      ['OG Organizr', 'og'],
      ['Modern', 'modern'],
    ]);
    await fireEvent.change(layout, { target: { value: 'modern' } });
    // The Organizr theme offers its dark and light color sets.
    expect(
      within(within(panel).getByRole('combobox', { name: 'Modern color set' }))
        .getAllByRole('option')
        .map((option) => [option.textContent, (option as HTMLOptionElement).value]),
    ).toEqual([
      ['Dark', 'dark'],
      ['Light', 'light'],
    ]);
    const themes = within(panel).getByRole('combobox', { name: 'Modern theme' });
    for (const label of [
      'Organizr',
      'Plex',
      'Jellyfin',
      'Emby',
      'Silo',
      'Nocturne',
      'Riot',
      'Tempest',
      'Circuit',
      'Prism',
    ]) {
      expect(within(themes).getByRole('option', { name: label })).toBeInTheDocument();
    }
    await fireEvent.change(themes, { target: { value: 'plex' } });
    const colors = within(panel).getByRole('combobox', { name: 'Modern color set' });
    expect(colors).toBeEnabled();
    await fireEvent.change(colors, { target: { value: 'light' } });

    await fireEvent.change(layout, { target: { value: 'og' } });
    expect(within(panel).getByRole('combobox', { name: 'Theme' })).toHaveValue('Nord');
    await fireEvent.change(layout, { target: { value: 'modern' } });
    expect(within(panel).getByRole('combobox', { name: 'Modern theme' })).toHaveValue('plex');
    expect(within(panel).getByRole('combobox', { name: 'Modern color set' })).toHaveValue('light');
  });

  it('modern_also_shows_the_og_theme_below_the_modern_ones_under_its_heading', async () => {
    const { panel, save, puts } = await open({ ...CONFIG, layout: 'modern' });

    const colors = within(panel).getByRole('combobox', { name: 'Modern color set' });
    const heading = within(panel).getByRole('heading', { name: 'OG Organizr' });
    const og = within(panel).getByRole('combobox', { name: 'Theme' });
    const follows = (first: Element, second: Element) =>
      (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(follows(colors, heading)).toBe(true);
    expect(follows(heading, og)).toBe(true);
    expect(og).toHaveValue('Organizr');
    expect(within(panel).getAllByLabelText('Theme', { exact: true })).toHaveLength(1);

    // Both themes change without a switch of the layout, and one Save sends both.
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern theme' }), {
      target: { value: 'plex' },
    });
    await fireEvent.change(og, { target: { value: 'Nord' } });
    await save();
    expect(puts()[0]?.body).toEqual({ theme: 'Nord', modernTheme: 'plex' });
  });

  it('og_shows_only_the_og_theme_without_the_og_heading', async () => {
    const { panel } = await open();

    expect(within(panel).getByRole('combobox', { name: 'Theme' })).toBeInTheDocument();
    expect(within(panel).queryByRole('heading', { name: 'OG Organizr' })).toBeNull();
    expect(within(panel).queryByRole('combobox', { name: 'Modern theme' })).toBeNull();
  });

  it('one_save_sends_the_look_and_a_color_together_and_reloads', async () => {
    const { sent, panel, save, puts } = await open();

    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    await fireEvent.input(within(panel).getByLabelText('Nav Bar Color', { selector: 'input' }), {
      target: { value: '#112233' },
    });
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Default layout' }), {
      target: { value: 'modern' },
    });
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern theme' }), {
      target: { value: 'emby' },
    });
    const before = sent.length;
    await save();

    expect(puts()).toHaveLength(1);
    expect(puts()[0]?.body).toEqual({
      headerColor: '#112233',
      theme: 'Nord',
      layout: 'modern',
      modernTheme: 'emby',
    });
    // The form loads again and hands the settings to the look, and the session loads again,
    // for the look of the page.
    const after = sent.slice(before).map((request) => `${request.method} ${request.path}`);
    expect(after).toEqual(expect.arrayContaining(['GET config', 'GET themes', 'GET launch']));
    expect(after.filter((request) => request === 'GET config')).toHaveLength(1);
    expect(after.some((request) => request.startsWith('GET settings/'))).toBe(false);
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save the layout' })).toBeNull();
  });

  it('co_admin_sees_the_layout_but_cannot_change_it', async () => {
    const { panel, save, puts } = await open({ ...CONFIG, layout: 'modern' }, 1);

    const layout = within(panel).getByRole('combobox', { name: 'Default layout' });
    expect(layout).toHaveValue('modern');
    expect(layout).toBeDisabled();
    expect(within(panel).getByRole('combobox', { name: 'Modern theme' })).toBeDisabled();
    expect(within(panel).getByRole('combobox', { name: 'Modern color set' })).toBeDisabled();
    // As in PHP's form, a Co-Admin changes the theme of the OG layout.
    const theme = within(panel).getByRole('combobox', { name: 'Theme' });
    expect(theme).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

    await fireEvent.change(theme, { target: { value: 'Nord' } });
    await save();
    expect(puts()[0]?.body).toEqual({ theme: 'Nord' });
  });

  it('a_co_admin_never_sends_the_first_set_fallback', async () => {
    const { panel } = await open(
      { layout: 'modern', modernTheme: 'silo', modernColorSet: 'dark' },
      1,
    );

    expect(within(panel).getByRole('combobox', { name: 'Modern color set' })).toHaveValue('ember');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('a_loaded_color_set_the_theme_does_not_have_shows_its_first_set_and_is_no_change', async () => {
    const { panel } = await open({
      layout: 'modern',
      modernTheme: 'silo',
      modernColorSet: 'dark',
    });

    expect(within(panel).getByRole('combobox', { name: 'Modern color set' })).toHaveValue('ember');
    // The server applies the first set anyway: nothing is changed on open.
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('a_failed_load_after_a_save_leaves_the_look_unchanged', async () => {
    let configs = 0;
    const { panel, save, puts } = await open();
    // The first load answered; the load after the save fails.
    const answer = globalThis.fetch;
    vi.stubGlobal('fetch', async (address: string, init?: RequestInit) => {
      if (String(address).endsWith('/api/v2/config') && (init?.method ?? 'GET') === 'GET') {
        configs += 1;
        return new Response(
          JSON.stringify({ response: { result: 'error', message: 'down', data: null } }),
          { status: 500 },
        );
      }
      return answer(address, init);
    });

    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Default layout' }), {
      target: { value: 'modern' },
    });
    expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();
    await save();
    expect(puts()[0]?.body).toEqual({ layout: 'modern' });
    expect(configs).toBe(1);
    // What was sent is stored: the form has no change left.
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  for (const [theme, first, labels] of [
    ['silo', 'ember', ['Ember', 'Ice', 'Rose']],
    ['nocturne', 'amethyst', ['Amethyst', 'Crimson', 'Moonlit']],
    ['riot', 'acid', ['Acid', 'Hotwire', 'Blacklight']],
    ['tempest', 'squall', ['Squall', 'Abyss', 'Gale']],
    ['circuit', 'matrix', ['Matrix', 'Synth', 'Amber']],
    ['prism', 'daydream', ['Daydream', 'Twilight', 'Candy']],
  ] as const) {
    it(`choosing_${theme}_moves_the_color_set_to_${first}`, async () => {
      const { panel, save, puts } = await open({ ...CONFIG, layout: 'modern' });

      await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern theme' }), {
        target: { value: theme },
      });
      const colors = within(panel).getByRole('combobox', { name: 'Modern color set' });
      expect(colors).toHaveValue(first);
      for (const label of labels) {
        expect(within(colors).getByRole('option', { name: label })).toBeInTheDocument();
      }
      expect(within(colors).queryByRole('option', { name: 'Dark' })).toBeNull();
      await save();
      expect(puts()[0]?.body).toEqual({ modernTheme: theme, modernColorSet: first });
    });
  }

  it('a_sample_shows_the_colors_of_the_chosen_theme_and_set', async () => {
    const { panel } = await open({ ...CONFIG, layout: 'modern' });

    const swatch = () => panel.querySelector('.org-swatch');
    expect(swatch()).toHaveAttribute('data-theme', 'm-organizr-dark');
    expect(swatch()).toHaveAttribute('aria-hidden', 'true');
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern color set' }), {
      target: { value: 'light' },
    });
    expect(swatch()).toHaveAttribute('data-theme', 'm-organizr-light');
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern theme' }), {
      target: { value: 'silo' },
    });
    expect(swatch()).toHaveAttribute('data-theme', 'm-silo-ember');
  });
});
