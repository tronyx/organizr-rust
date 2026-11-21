import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import AppearanceSettings from './AppearanceSettings.svelte';
import ThemeMarketplace from './ThemeMarketplace.svelte';

const LAUNCH = {
  data: {
    user: { username: 'zed', groupID: 0, loggedin: true, locked: 0 },
    tabs: [],
    categories: [],
    plugins: {},
    theme: 'Organizr',
    appearance: { title: 'Organizr' },
    settings: { misc: {} },
    status: { status: 'ok' },
  },
};

function market(nordInstalled: boolean) {
  return {
    data: {
      Nord: {
        author: 'GilbN',
        category: 'Community',
        description: 'A Nord colored theme for Organizr ',
        version: '1.0.2',
        installed: nordInstalled,
        status: nordInstalled ? 'Up to date' : 'Not Installed',
      },
      Space_Gray: {
        author: 'GilbN',
        category: 'Community',
        version: '2.0.2',
        installed: false,
        status: 'Not Installed',
      },
    },
  };
}

describe('the theme marketplace', () => {
  it('lists the themes with a preview, and removes only an installed one', async () => {
    stubApi({ 'GET themes/marketplace': market(false) });
    render(ThemeMarketplace, { ondone: vi.fn() });

    const name = await screen.findByText('Space Gray', { selector: '.font-medium' });
    const row = name.closest('tr') as HTMLElement;
    expect(
      within(row).getByRole('img', { name: 'Preview of the theme Space Gray' }),
    ).toHaveAttribute('data-theme', 'org-space-gray');
    expect(within(row).getByText('Not Installed')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Remove Nord' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Install Nord' })).toBeEnabled();
  });

  it('installs a theme by its name in lower case and shows the new state', async () => {
    let installed = false;
    const ondone = vi.fn();
    const sent = stubApi({
      'GET themes/marketplace': () => market(installed),
      'POST themes/manage/nord': () => {
        installed = true;
        return { message: 'Theme installed' };
      },
      'GET launch': LAUNCH,
    });
    render(ThemeMarketplace, { ondone });

    await fireEvent.click(await screen.findByRole('button', { name: 'Install Nord' }));

    await waitFor(() => expect(screen.getByRole('button', { name: 'Remove Nord' })).toBeEnabled());
    expect(
      sent.some((request) => request.method === 'POST' && request.path === 'themes/manage/nord'),
    ).toBe(true);
    expect(ondone).toHaveBeenCalledWith('The theme Nord is installed.');
  });

  it('removes a theme', async () => {
    let installed = true;
    const sent = stubApi({
      'GET themes/marketplace': () => market(installed),
      'DELETE themes/manage/nord': () => {
        installed = false;
        return { message: 'Theme removed' };
      },
      'GET launch': LAUNCH,
    });
    render(ThemeMarketplace, { ondone: vi.fn() });

    await fireEvent.click(await screen.findByRole('button', { name: 'Remove Nord' }));

    await waitFor(() => expect(screen.getByRole('button', { name: 'Remove Nord' })).toBeDisabled());
    expect(sent.some((request) => request.method === 'DELETE')).toBe(true);
  });

  it('shows why the list cannot be loaded', async () => {
    stubApi({ 'GET themes/marketplace': { status: 401, message: 'Not Authorized' } });
    render(ThemeMarketplace, { ondone: vi.fn() });

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
  });
});

describe('the appearance settings', () => {
  it('offer the themes of the install that have a native version, with a preview', async () => {
    stubApi({
      'GET themes': {
        data: {
          Blue: { name: 'Blue' },
          Organizr: { name: 'Organizr' },
          Nord: { name: 'Nord' },
          Other: { name: 'Other' },
        },
      },
      'GET config': { data: { theme: 'Nord', headerColor: '', customCss: '' } },
    });
    render(AppearanceSettings, { ondone: vi.fn() });

    const select = await screen.findByLabelText('Theme');
    const options = within(select)
      .getAllByRole('option')
      .map((option) => option.textContent);
    expect(options).toEqual(['Blue', 'Organizr', 'Nord']);
    expect(screen.getByRole('img', { name: 'Preview of the theme Nord' })).toHaveAttribute(
      'data-theme',
      'org-nord',
    );

    await fireEvent.change(select, { target: { value: 'Blue' } });
    expect(screen.getByRole('img', { name: 'Preview of the theme Blue' })).toBeInTheDocument();
  });

  it('save a colour and say when a colour is wrong', async () => {
    const sent = stubApi({
      'GET themes': { data: { Organizr: { name: 'Organizr' } } },
      'GET config': { data: { theme: 'Organizr', headerColor: '' } },
      'PUT config': { message: 'Config items updated' },
      'GET launch': LAUNCH,
    });
    render(AppearanceSettings, { ondone: vi.fn() });

    const field = await screen.findByLabelText('Colour of the bar at the top');
    await fireEvent.input(field, { target: { value: 'red; color: blue' } });
    expect(field).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText(/This is no colour/)).toBeInTheDocument();

    await fireEvent.input(field, { target: { value: '#112233' } });
    expect(field).toHaveAttribute('aria-invalid', 'false');
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
        headerColor: '#112233',
      }),
    );
  });
});
