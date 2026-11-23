import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

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

    expect(screen.getByRole('radio', { name: 'Original' })).toBeChecked();
    const themes = screen.getByRole('combobox', { name: 'Theme' });
    expect(themes).toHaveValue('Organizr');
    expect(themes.querySelectorAll('option')).toHaveLength(13);
    expect(screen.getByRole('option', { name: 'Space Gray' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Light' })).not.toBeChecked();
  });

  it('shows_the_modern_choices_on_modern', async () => {
    await open(MODERN);

    expect(screen.getByRole('radio', { name: 'Modern' })).toBeChecked();
    const themes = screen.getByRole('combobox', { name: 'Theme' });
    expect(themes.querySelectorAll('option')).toHaveLength(1);
    expect(screen.getByRole('option', { name: 'Organizr' })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: 'Space Gray' })).not.toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
  });

  it('choosing_sends_the_layout_theme_and_colour_set', async () => {
    const sent = await open(OG);
    const puts = () => sent.filter((request) => request.method === 'PUT');

    await fireEvent.change(screen.getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    await waitFor(() => expect(puts()).toHaveLength(1));
    expect(puts()[0]?.body).toEqual({ layout: 'og', theme: 'Nord', colourSet: 'dark' });

    await fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
    await waitFor(() => expect(puts()).toHaveLength(2));
    expect(puts()[1]?.body).toEqual({ layout: 'og', theme: 'Organizr', colourSet: 'light' });

    await fireEvent.click(screen.getByRole('radio', { name: 'Modern' }));
    await waitFor(() => expect(puts()).toHaveLength(3));
    expect(puts()[2]?.body).toEqual({ layout: 'modern' });
    // Each change loads the look again.
    await waitFor(() =>
      expect(sent.filter((request) => request.path === 'launch').length).toBeGreaterThanOrEqual(4),
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

    await fireEvent.change(screen.getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    expect(await screen.findByRole('alert')).toHaveTextContent('theme is not valid');
  });
});
