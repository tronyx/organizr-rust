// The group "Style & Themes" of the Appearance settings: PHP's group "Colors & Themes",
// with the default layout and the theme of each layout at its top, and one Save.
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import AppearanceSettings from './AppearanceSettings.svelte';

const CONFIG = { layout: 'og', modernTheme: 'organizr', modernColourSet: 'dark' };

/** PHP's group, as `GET settings/appearance` gives it, shortened. */
const COLOURS = [
  { type: 'notice', override: 12, name: null, notice: 'info', title: 'Attention', body: '' },
  { type: 'button', label: 'Reset Colors', name: '', text: 'Reset' },
  {
    type: 'input',
    label: 'Nav Bar Color',
    class: 'pick-a-color-custom-options',
    name: 'headerColor',
    value: '',
  },
  {
    type: 'select',
    label: 'Theme',
    name: 'theme',
    value: 'Organizr',
    options: [
      { name: 'Organizr', value: 'Organizr' },
      { name: 'Nord', value: 'Nord' },
    ],
  },
  {
    type: 'select',
    label: 'Style',
    name: 'style',
    value: 'dark',
    options: [
      { name: 'Light', value: 'light' },
      { name: 'Dark', value: 'dark' },
      { name: 'Horizontal', value: 'horizontal' },
    ],
  },
];

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

/** Opens the page on the group, as the Admin (0) or a Co-Admin (1). */
async function open(config: Record<string, unknown> = CONFIG, groupID = 0) {
  const sent = stubApi({
    'GET launch': launchOf(groupID),
    'GET settings/appearance': {
      data: {
        'Top Bar': [{ type: 'input', label: 'Title', name: 'title', value: 'Organizr' }],
        'Colors & Themes': COLOURS,
      },
    },
    'GET config': { data: config },
    'PUT config': {},
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
  it('replaces_the_layout_tab_and_keeps_its_place', async () => {
    await open();

    const tabs = screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    expect(tabs.slice(0, 2)).toEqual(['Top Bar', 'Style & Themes']);
    expect(screen.queryByRole('tab', { name: 'Layout' })).toBeNull();
    expect(screen.queryByRole('tab', { name: 'Colors & Themes' })).toBeNull();
  });

  it('shows_the_notice_then_default_layout_then_the_colours', async () => {
    const { panel } = await open();

    const notice = within(panel).getByRole('heading', { name: 'Attention' });
    const layout = within(panel).getByRole('combobox', { name: 'Default layout' });
    const heading = within(panel).getByRole('heading', {
      name: 'Colours of the OG Organizr layout',
    });
    const colour = within(panel).getByLabelText('Nav Bar Color', { selector: 'input' });
    const follows = (first: Element, second: Element) =>
      (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(follows(notice, layout)).toBe(true);
    expect(within(panel).getAllByRole('combobox')[0]).toBe(layout);
    expect(follows(layout, heading)).toBe(true);
    expect(follows(heading, colour)).toBe(true);
    expect(within(panel).getByRole('button', { name: 'Reset' })).toBeInTheDocument();
    expect(panel).toHaveTextContent('apply to the OG Organizr layout only');
  });

  it('og_shows_theme_with_the_options_of_php_once_and_no_style', async () => {
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
    // PHP's own field Theme is not shown a second time. PHP's Style has no effect (D-92),
    // so the page has no field for it.
    expect(within(panel).getAllByLabelText('Theme', { exact: true })).toHaveLength(1);
    expect(within(panel).queryByLabelText('Style', { exact: true })).toBeNull();
    expect(within(panel).queryByRole('combobox', { name: 'Modern theme' })).toBeNull();
    expect(within(panel).queryByRole('combobox', { name: 'Modern colour set' })).toBeNull();
    expect(panel.querySelector('.org-swatch')).toHaveAttribute('data-theme', 'organizr');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('modern_shows_the_modern_selects_and_switching_back_keeps_the_values', async () => {
    const { panel } = await open();

    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    const layout = within(panel).getByRole('combobox', { name: 'Default layout' });
    await fireEvent.change(layout, { target: { value: 'modern' } });
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
    const colours = within(panel).getByRole('combobox', { name: 'Modern colour set' });
    expect(colours).toBeEnabled();
    await fireEvent.change(colours, { target: { value: 'light' } });

    await fireEvent.change(layout, { target: { value: 'og' } });
    expect(within(panel).getByRole('combobox', { name: 'Theme' })).toHaveValue('Nord');
    await fireEvent.change(layout, { target: { value: 'modern' } });
    expect(within(panel).getByRole('combobox', { name: 'Modern theme' })).toHaveValue('plex');
    expect(within(panel).getByRole('combobox', { name: 'Modern colour set' })).toHaveValue('light');
  });

  it('modern_also_shows_the_og_theme_below_the_modern_ones_under_its_heading', async () => {
    const { panel, save, puts } = await open({ ...CONFIG, layout: 'modern' });

    const colours = within(panel).getByRole('combobox', { name: 'Modern colour set' });
    const heading = within(panel).getByRole('heading', { name: 'OG Organizr' });
    const og = within(panel).getByRole('combobox', { name: 'Theme' });
    const follows = (first: Element, second: Element) =>
      (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(follows(colours, heading)).toBe(true);
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

  it('one_save_sends_the_look_and_a_colour_together_and_reloads', async () => {
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
    // Both parts load again, and so does the session, for the look of the page.
    const after = sent.slice(before).map((request) => `${request.method} ${request.path}`);
    expect(after).toEqual(
      expect.arrayContaining(['GET settings/appearance', 'GET config', 'GET launch']),
    );
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save the layout' })).toBeNull();
  });

  it('co_admin_sees_the_layout_but_cannot_change_it', async () => {
    const { panel, save, puts } = await open({ ...CONFIG, layout: 'modern' }, 1);

    const layout = within(panel).getByRole('combobox', { name: 'Default layout' });
    expect(layout).toHaveValue('modern');
    expect(layout).toBeDisabled();
    expect(within(panel).getByRole('combobox', { name: 'Modern theme' })).toBeDisabled();
    expect(within(panel).getByRole('combobox', { name: 'Modern colour set' })).toBeDisabled();
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
      { layout: 'modern', modernTheme: 'silo', modernColourSet: 'dark' },
      1,
    );

    expect(within(panel).getByRole('combobox', { name: 'Modern colour set' })).toHaveValue('ember');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('a_loaded_colour_set_the_theme_does_not_have_shows_its_first_set_and_is_no_change', async () => {
    const { panel } = await open({
      layout: 'modern',
      modernTheme: 'silo',
      modernColourSet: 'dark',
    });

    expect(within(panel).getByRole('combobox', { name: 'Modern colour set' })).toHaveValue('ember');
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
    it(`choosing_${theme}_moves_the_colour_set_to_${first}`, async () => {
      const { panel, save, puts } = await open({ ...CONFIG, layout: 'modern' });

      await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern theme' }), {
        target: { value: theme },
      });
      const colours = within(panel).getByRole('combobox', { name: 'Modern colour set' });
      expect(colours).toHaveValue(first);
      for (const label of labels) {
        expect(within(colours).getByRole('option', { name: label })).toBeInTheDocument();
      }
      expect(within(colours).queryByRole('option', { name: 'Dark' })).toBeNull();
      await save();
      expect(puts()[0]?.body).toEqual({ modernTheme: theme, modernColourSet: first });
    });
  }

  it('a_sample_shows_the_colours_of_the_chosen_theme_and_set', async () => {
    const { panel } = await open({ ...CONFIG, layout: 'modern' });

    const swatch = () => panel.querySelector('.org-swatch');
    expect(swatch()).toHaveAttribute('data-theme', 'm-organizr-dark');
    expect(swatch()).toHaveAttribute('aria-hidden', 'true');
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern colour set' }), {
      target: { value: 'light' },
    });
    expect(swatch()).toHaveAttribute('data-theme', 'm-organizr-light');
    await fireEvent.change(within(panel).getByRole('combobox', { name: 'Modern theme' }), {
      target: { value: 'silo' },
    });
    expect(swatch()).toHaveAttribute('data-theme', 'm-silo-ember');
  });
});
