import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { session } from '../lib/session.svelte';
import { stubApi, type CannedAnswer } from '../test-helpers';
import LookPicker from './LookPicker.svelte';

function launch(look: Record<string, string>): CannedAnswer {
  return {
    data: {
      user: { username: 'robin', group: 'Admin', groupID: 0, userID: 5, loggedin: true, locked: 0 },
      look,
      tabs: [],
      categories: [],
      plugins: { includes: {} },
      appearance: { title: 'Organizr' },
      settings: {
        sso: { plex: { oAuthEnabled: false } },
        homepage: { refresh: {}, options: {} },
        misc: {},
        ping: { auth: 999, adminRefresh: 60000, everyoneRefresh: 60000 },
      },
      status: { status: 'ok' },
    },
  };
}

const OG = { layout: 'og', theme: 'Organizr', colourSet: 'dark', source: 'default' };
const MODERN = { layout: 'modern', theme: 'organizr', colourSet: 'dark', source: 'user' };

async function open(look: Record<string, string>, extra: Record<string, CannedAnswer> = {}) {
  const sent = stubApi({ 'GET launch': launch(look), 'PUT user/look': {}, ...extra });
  await session.load();
  render(LookPicker);
  return sent;
}

describe('LookPicker', () => {
  it('shows_the_og_choices_on_og', async () => {
    await open(OG);

    expect(screen.getByRole('radio', { name: 'OG Organizr' })).toBeChecked();
    const themes = screen.getByRole('group', { name: 'Theme' });
    expect(within(themes).getByRole('radio', { name: 'Organizr' })).toBeChecked();
    expect(within(themes).getAllByRole('radio')).toHaveLength(13);
    expect(screen.getByRole('radio', { name: 'Space Gray' })).toBeInTheDocument();
    // The Colours of the OG layout do nothing (its `style` setting has no effect).
    expect(screen.queryByRole('group', { name: 'Colours' })).not.toBeInTheDocument();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('shows_the_modern_choices_on_modern', async () => {
    await open(MODERN);

    expect(screen.getByRole('radio', { name: 'Modern' })).toBeChecked();
    const themes = screen.getByRole('group', { name: 'Theme' });
    const labels = within(themes)
      .getAllByRole('radio')
      .map((radio) => radio.closest('label')?.textContent.trim());
    expect(labels).toEqual([
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
    ]);
    expect(within(themes).getByRole('radio', { name: 'Organizr' })).toBeChecked();
    expect(screen.queryByRole('radio', { name: 'Space Gray' })).not.toBeInTheDocument();
    // The theme has a dark and a light colour set; either can be chosen.
    expect(screen.getByRole('group', { name: 'Colours' })).toBeInTheDocument();
    expect(screen.getAllByRole('radio', { name: /^(Dark|Light)$/ })).toHaveLength(2);
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Light' })).not.toBeChecked();
    expect(screen.getByRole('radio', { name: 'Light' })).toBeEnabled();
  });

  it('choosing_a_brand_theme_keeps_the_colour_set', async () => {
    const sent = await open({ ...MODERN, colourSet: 'light' });

    await fireEvent.click(screen.getByRole('radio', { name: 'Jellyfin' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'jellyfin',
      colourSet: 'light',
    });
  });

  it('choosing_silo_moves_to_its_first_colour_set', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Silo' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    // Silo has no `dark` set: the first of its sets takes its place.
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'silo',
      colourSet: 'ember',
    });
  });

  it('silo_offers_its_three_accent_sets', async () => {
    await open({ ...MODERN, theme: 'silo', colourSet: 'ice' });

    expect(screen.getByRole('radio', { name: 'Ice' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Ember' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Rose' })).toBeEnabled();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('choosing_nocturne_moves_to_amethyst_and_offers_its_sets', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Nocturne' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'nocturne',
      colourSet: 'amethyst',
    });
  });

  it('nocturne_offers_its_three_colour_sets', async () => {
    await open({ ...MODERN, theme: 'nocturne', colourSet: 'moonlit' });

    expect(screen.getByRole('radio', { name: 'Moonlit' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Amethyst' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Crimson' })).toBeEnabled();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('choosing_riot_moves_to_acid_and_offers_its_sets', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Riot' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'riot',
      colourSet: 'acid',
    });
  });

  it('riot_offers_its_three_colour_sets', async () => {
    await open({ ...MODERN, theme: 'riot', colourSet: 'blacklight' });

    expect(screen.getByRole('radio', { name: 'Blacklight' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Acid' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Hotwire' })).toBeEnabled();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('choosing_tempest_moves_to_squall_and_offers_its_sets', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Tempest' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'tempest',
      colourSet: 'squall',
    });
  });

  it('tempest_offers_its_three_colour_sets', async () => {
    await open({ ...MODERN, theme: 'tempest', colourSet: 'gale' });

    expect(screen.getByRole('radio', { name: 'Gale' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Squall' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Abyss' })).toBeEnabled();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('choosing_circuit_moves_to_matrix_and_offers_its_sets', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Circuit' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'circuit',
      colourSet: 'matrix',
    });
  });

  it('circuit_offers_its_three_colour_sets', async () => {
    await open({ ...MODERN, theme: 'circuit', colourSet: 'amber' });

    expect(screen.getByRole('radio', { name: 'Amber' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Matrix' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Synth' })).toBeEnabled();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('choosing_prism_moves_to_daydream_and_offers_its_sets', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Prism' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'prism',
      colourSet: 'daydream',
    });
  });

  it('prism_offers_its_three_colour_sets', async () => {
    await open({ ...MODERN, theme: 'prism', colourSet: 'candy' });

    expect(screen.getByRole('radio', { name: 'Candy' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Daydream' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Twilight' })).toBeEnabled();
    expect(screen.queryByRole('radio', { name: 'Light' })).not.toBeInTheDocument();
  });

  it('choosing_light_sends_the_light_colour_set', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      theme: 'organizr',
      colourSet: 'light',
    });
  });

  it('choosing_sends_the_layout_theme_and_colour_set', async () => {
    const sent = await open(OG);
    const puts = () => sent.filter((request) => request.method === 'PUT');

    await fireEvent.click(screen.getByRole('radio', { name: 'Nord' }));
    await waitFor(() => expect(puts()).toHaveLength(1));
    expect(puts()[0]?.body).toEqual({ layout: 'og', theme: 'Nord', colourSet: 'dark' });

    await fireEvent.click(screen.getByRole('radio', { name: 'Modern' }));
    await waitFor(() => expect(puts()).toHaveLength(2));
    expect(puts()[1]?.body).toEqual({ layout: 'modern' });
    // Each change loads the look again.
    await waitFor(() =>
      expect(sent.filter((request) => request.path === 'launch').length).toBeGreaterThanOrEqual(3),
    );
  });

  it('use_the_default_sends_layout_null', async () => {
    const sent = await open(MODERN);

    await fireEvent.click(screen.getByRole('button', { name: 'Use the default' }));
    await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: null,
      theme: null,
      colourSet: null,
    });
  });

  it('failure_is_shown', async () => {
    await open(OG, { 'PUT user/look': { status: 422, message: 'theme is not valid' } });

    await fireEvent.click(screen.getByRole('radio', { name: 'Nord' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('theme is not valid');
  });

  it('after_a_failed_save_the_controls_show_the_saved_look_again', async () => {
    await open(OG, { 'PUT user/look': { status: 422, message: 'theme is not valid' } });

    await fireEvent.click(screen.getByRole('radio', { name: 'Nord' }));
    await screen.findByRole('alert');
    expect(screen.getByRole('radio', { name: 'Organizr' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Nord' })).not.toBeChecked();

    await fireEvent.click(screen.getByRole('radio', { name: 'Modern' }));
    await waitFor(() => expect(screen.getAllByRole('alert')).toHaveLength(1));
    await waitFor(() => expect(screen.getByRole('radio', { name: 'OG Organizr' })).toBeChecked());
    expect(screen.getByRole('radio', { name: 'Modern' })).not.toBeChecked();
  });

  it('the_radios_stay_enabled_and_the_group_is_busy_while_a_save_is_in_flight', async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => (release = resolve));
    const sent = await open(OG);
    const answer = globalThis.fetch;
    // The save waits until the test lets it go.
    vi.stubGlobal('fetch', async (address: string, init?: RequestInit) => {
      if (init?.method === 'PUT') {
        await gate;
      }
      return answer(address, init);
    });
    const themes = screen.getByRole('group', { name: 'Theme' });
    expect(themes).toHaveAttribute('aria-busy', 'false');

    const nord = screen.getByRole('radio', { name: 'Nord' });
    nord.focus();
    await fireEvent.click(nord);
    await waitFor(() => expect(themes).toHaveAttribute('aria-busy', 'true'));
    // A disabled radio would lose the focus and the arrow keys.
    expect(nord).toBeEnabled();
    expect(nord).toHaveFocus();
    expect(screen.getByRole('radio', { name: 'Plex' })).toBeEnabled();
    expect(screen.getByRole('radio', { name: 'Modern' })).toBeEnabled();

    release();
    await waitFor(() => expect(themes).toHaveAttribute('aria-busy', 'false'));
    expect(sent.some((request) => request.method === 'PUT')).toBe(true);
    expect(nord).toHaveFocus();
  });

  it('choices_made_during_a_save_send_only_the_last_one_next', async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => (release = resolve));
    const sent = await open(OG);
    const puts = () => sent.filter((request) => request.method === 'PUT');
    const answer = globalThis.fetch;
    let first = true;
    // The first save waits until the test lets it go.
    vi.stubGlobal('fetch', async (address: string, init?: RequestInit) => {
      if (init?.method === 'PUT' && first) {
        first = false;
        await gate;
      }
      return answer(address, init);
    });

    // As the arrow keys do: each step checks the next radio.
    await fireEvent.click(screen.getByRole('radio', { name: 'Nord' }));
    await fireEvent.click(screen.getByRole('radio', { name: 'Plex' }));
    const last = screen.getByRole('radio', { name: 'Space Gray' });
    last.focus();
    await fireEvent.click(last);
    // The controls show the last choice while it waits.
    expect(last).toBeChecked();

    release();
    await waitFor(() => expect(puts()).toHaveLength(2));
    expect(puts().map((request) => (request.body as { theme: string }).theme)).toEqual([
      'Nord',
      'Space_Gray',
    ]);
    await waitFor(() =>
      expect(screen.getByRole('group', { name: 'Theme' })).toHaveAttribute('aria-busy', 'false'),
    );
    expect(last).toHaveFocus();
  });

  it('after_a_failed_save_the_focus_stays_on_the_checked_radio', async () => {
    await open(OG, { 'PUT user/look': { status: 422, message: 'theme is not valid' } });

    const nord = screen.getByRole('radio', { name: 'Nord' });
    nord.focus();
    await fireEvent.click(nord);
    await screen.findByRole('alert');
    // The controls are drawn again with the saved look; the focus stays in the group.
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Organizr' })).toHaveFocus());
  });

  /** The `data-theme` of the colour sample of a radio, from the label that holds both. */
  function swatchOf(name: string): string | null | undefined {
    return screen
      .getByRole('radio', { name })
      .closest('label')
      ?.querySelector('.org-swatch')
      ?.getAttribute('data-theme');
  }

  it('every_og_theme_shows_a_sample_of_its_colours', async () => {
    await open(OG);

    const radios = within(screen.getByRole('group', { name: 'Theme' })).getAllByRole('radio');
    expect(radios).toHaveLength(13);
    for (const radio of radios) {
      expect(radio.closest('label')?.querySelectorAll('.org-swatch')).toHaveLength(1);
    }
    expect(swatchOf('Organizr')).toBe('organizr');
    expect(swatchOf('Nord')).toBe('org-nord');
    expect(swatchOf('Space Gray')).toBe('org-space-gray');
    expect(swatchOf('Plex')).toBe('org-plex');
  });

  it('every_modern_theme_shows_a_sample_in_the_colour_set_it_would_get', async () => {
    await open({ ...MODERN, colourSet: 'light' });

    const radios = within(screen.getByRole('group', { name: 'Theme' })).getAllByRole('radio');
    expect(radios).toHaveLength(10);
    for (const radio of radios) {
      expect(radio.closest('label')?.querySelectorAll('.org-swatch')).toHaveLength(1);
    }
    // A theme with the current set shows it; one without shows its first set.
    expect(swatchOf('Organizr')).toBe('m-organizr-light');
    expect(swatchOf('Plex')).toBe('m-plex-light');
    expect(swatchOf('Silo')).toBe('m-silo-ember');
    expect(swatchOf('Nocturne')).toBe('m-nocturne-amethyst');
    expect(swatchOf('Prism')).toBe('m-prism-daydream');
  });

  it('a_theme_with_the_current_set_shows_that_set', async () => {
    await open({ ...MODERN, theme: 'silo', colourSet: 'rose' });

    expect(screen.getByRole('radio', { name: 'Silo' })).toBeChecked();
    expect(swatchOf('Silo')).toBe('m-silo-rose');
    // Plex has no `rose`: its first set.
    expect(swatchOf('Plex')).toBe('m-plex-dark');
  });

  it('every_colour_set_shows_a_sample_of_its_colours', async () => {
    await open({ ...MODERN, theme: 'nocturne', colourSet: 'crimson' });

    expect(swatchOf('Amethyst')).toBe('m-nocturne-amethyst');
    expect(swatchOf('Crimson')).toBe('m-nocturne-crimson');
    expect(swatchOf('Moonlit')).toBe('m-nocturne-moonlit');
  });

  it('the_samples_are_hidden_from_assistive_technology', async () => {
    await open(MODERN);

    const swatches = document.querySelectorAll('.org-swatch');
    // Ten themes and two colour sets.
    expect(swatches).toHaveLength(12);
    for (const swatch of swatches) {
      expect(swatch).toHaveAttribute('aria-hidden', 'true');
      expect(swatch.querySelectorAll('.org-swatch-dot')).toHaveLength(3);
    }
    // The name of each radio is its label alone.
    expect(screen.getByRole('radio', { name: 'Plex' })).toBeInTheDocument();
  });

  it('the_theme_radios_have_their_own_group_name', async () => {
    await open(MODERN);

    const names = new Set(
      ['Modern', 'Plex', 'Dark'].map((name) =>
        screen.getByRole('radio', { name }).getAttribute('name'),
      ),
    );
    expect(names.size).toBe(3);
  });
});
