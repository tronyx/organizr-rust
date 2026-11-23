import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
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
    const themes = screen.getByRole('combobox', { name: 'Theme' });
    expect(themes).toHaveValue('Organizr');
    expect(themes.querySelectorAll('option')).toHaveLength(13);
    expect(screen.getByRole('option', { name: 'Space Gray' })).toBeInTheDocument();
    // The Colours of the OG layout do nothing (its `style` setting has no effect).
    expect(screen.queryByRole('group', { name: 'Colours' })).not.toBeInTheDocument();
    expect(screen.queryByRole('radio', { name: 'Dark' })).not.toBeInTheDocument();
  });

  it('shows_the_modern_choices_on_modern', async () => {
    await open(MODERN);

    expect(screen.getByRole('radio', { name: 'Modern' })).toBeChecked();
    const themes = screen.getByRole('combobox', { name: 'Theme' });
    expect(themes.querySelectorAll('option')).toHaveLength(1);
    expect(screen.getByRole('option', { name: 'Organizr' })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: 'Space Gray' })).not.toBeInTheDocument();
    // One colour set is offered, so the group is shown but cannot change.
    expect(screen.getByRole('group', { name: 'Colours' })).toBeInTheDocument();
    expect(screen.getAllByRole('radio', { name: /^(Dark|Light)$/ })).toHaveLength(1);
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeDisabled();
    expect(screen.queryByRole('radio', { name: 'Light' })).not.toBeInTheDocument();
  });

  it('choosing_sends_the_layout_theme_and_colour_set', async () => {
    const sent = await open(OG);
    const puts = () => sent.filter((request) => request.method === 'PUT');

    await fireEvent.change(screen.getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
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

    await fireEvent.change(screen.getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    expect(await screen.findByRole('alert')).toHaveTextContent('theme is not valid');
  });

  it('after_a_failed_save_the_controls_show_the_saved_look_again', async () => {
    await open(OG, { 'PUT user/look': { status: 422, message: 'theme is not valid' } });

    const themes = screen.getByRole('combobox', { name: 'Theme' });
    await fireEvent.change(themes, { target: { value: 'Nord' } });
    await screen.findByRole('alert');
    expect(screen.getByRole('combobox', { name: 'Theme' })).toHaveValue('Organizr');

    await fireEvent.click(screen.getByRole('radio', { name: 'Modern' }));
    await waitFor(() => expect(screen.getAllByRole('alert')).toHaveLength(1));
    await waitFor(() => expect(screen.getByRole('radio', { name: 'OG Organizr' })).toBeChecked());
    expect(screen.getByRole('radio', { name: 'Modern' })).not.toBeChecked();
  });

  it('the_controls_are_disabled_while_a_save_is_in_flight', async () => {
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

    await fireEvent.change(screen.getByRole('combobox', { name: 'Theme' }), {
      target: { value: 'Nord' },
    });
    await waitFor(() => expect(screen.getByRole('combobox', { name: 'Theme' })).toBeDisabled());
    expect(screen.getByRole('radio', { name: 'OG Organizr' })).toBeDisabled();
    expect(screen.getByRole('radio', { name: 'Modern' })).toBeDisabled();

    release();
    await waitFor(() => expect(screen.getByRole('combobox', { name: 'Theme' })).toBeEnabled());
    expect(sent.some((request) => request.method === 'PUT')).toBe(true);
  });
});
