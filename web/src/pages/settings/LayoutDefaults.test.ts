import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import LayoutDefaults from './LayoutDefaults.svelte';

const CONFIG = { layout: 'og', modernTheme: 'organizr', modernColourSet: 'dark' };

describe('LayoutDefaults', () => {
  it('saves_the_three_settings', async () => {
    const sent = stubApi({
      'GET config': { data: CONFIG },
      'PUT config': {},
      'GET launch': { status: 500, message: 'not needed here' },
    });
    const ondone = vi.fn();
    render(LayoutDefaults, { canChange: true, ondone });

    const layout = await screen.findByRole('combobox', { name: 'Default layout' });
    expect(layout).toHaveValue('og');
    expect(screen.getByRole('option', { name: 'OG Organizr' })).toBeInTheDocument();
    const save = screen.getByRole('button', { name: 'Save the layout' });
    expect(save).toBeDisabled();

    await fireEvent.change(layout, { target: { value: 'modern' } });
    const themes = screen.getByRole('combobox', { name: 'Modern theme' });
    for (const label of ['Organizr', 'Plex', 'Jellyfin', 'Emby', 'Silo']) {
      expect(screen.getByRole('option', { name: label })).toBeInTheDocument();
    }
    await fireEvent.change(themes, { target: { value: 'plex' } });
    const colours = screen.getByRole('combobox', { name: 'Modern colour set' });
    expect(colours).toBeEnabled();
    expect(screen.getByRole('option', { name: 'Dark' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Light' })).toBeInTheDocument();
    await fireEvent.change(colours, { target: { value: 'light' } });
    await fireEvent.click(save);

    await waitFor(() => expect(ondone).toHaveBeenCalled());
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      modernTheme: 'plex',
      modernColourSet: 'light',
    });
  });

  it('choosing_silo_moves_the_colour_set_to_ember', async () => {
    const sent = stubApi({
      'GET config': { data: { ...CONFIG, layout: 'modern' } },
      'PUT config': {},
      'GET launch': { status: 500, message: 'not needed here' },
    });
    const ondone = vi.fn();
    render(LayoutDefaults, { canChange: true, ondone });

    const themes = await screen.findByRole('combobox', { name: 'Modern theme' });
    await fireEvent.change(themes, { target: { value: 'silo' } });
    // Silo has no `dark` set, so the colour set moves to its first one.
    const colours = screen.getByRole('combobox', { name: 'Modern colour set' });
    expect(colours).toHaveValue('ember');
    for (const label of ['Ember', 'Ice', 'Rose']) {
      expect(screen.getByRole('option', { name: label })).toBeInTheDocument();
    }
    expect(screen.queryByRole('option', { name: 'Dark' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Save the layout' }));

    await waitFor(() => expect(ondone).toHaveBeenCalled());
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      modernTheme: 'silo',
      modernColourSet: 'ember',
    });
  });

  it('a_loaded_colour_set_the_theme_does_not_have_shows_its_first_set', async () => {
    const sent = stubApi({
      'GET config': {
        data: { layout: 'modern', modernTheme: 'silo', modernColourSet: 'dark' },
      },
      'PUT config': {},
      'GET launch': { status: 500, message: 'not needed here' },
    });
    const ondone = vi.fn();
    render(LayoutDefaults, { canChange: true, ondone });

    const colours = await screen.findByRole('combobox', { name: 'Modern colour set' });
    expect(colours).toHaveValue('ember');
    await fireEvent.click(screen.getByRole('button', { name: 'Save the layout' }));

    await waitFor(() => expect(ondone).toHaveBeenCalled());
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      layout: 'modern',
      modernTheme: 'silo',
      modernColourSet: 'ember',
    });
  });

  it('co_admin_sees_but_cannot_change', async () => {
    const sent = stubApi({ 'GET config': { data: { ...CONFIG, layout: 'modern' } } });
    render(LayoutDefaults, { canChange: false, ondone: vi.fn() });

    const layout = await screen.findByRole('combobox', { name: 'Default layout' });
    expect(layout).toHaveValue('modern');
    expect(layout).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Modern theme' })).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Modern colour set' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save the layout' })).not.toBeInTheDocument();
    expect(sent.some((request) => request.method === 'PUT')).toBe(false);
  });
});
