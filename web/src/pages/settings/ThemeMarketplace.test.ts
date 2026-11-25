import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
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
