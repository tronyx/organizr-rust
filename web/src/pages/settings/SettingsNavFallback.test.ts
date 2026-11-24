import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import Settings from './Settings.svelte';

// The chunk of the side menu does not load (a network error, or a new version of the
// install that replaced the files): the import fails.
vi.mock('./SettingsNav.svelte', () => {
  throw new Error('Failed to fetch dynamically imported module');
});

describe('Settings on the modern layout, when the side menu does not load', () => {
  afterEach(() => {
    cleanup();
    session.launch = null;
    delete document.documentElement.dataset.layout;
  });

  it('the_tabs_of_og_take_its_place', async () => {
    stubApi({
      'GET tabs': { data: { tabs: [], categories: [], groups: [] } },
      'GET users': { data: [] },
      'GET launch': {
        data: {
          categories: [],
          tabs: [],
          user: null,
          version: '0.1.0',
          appearance: { title: 'Organizr' },
          plugins: {},
          look: { layout: 'modern', theme: 'organizr', colorSet: 'dark', source: 'user' },
          settings: {
            sso: { plex: { oAuthEnabled: false } },
            homepage: { refresh: {}, options: { healthChecksTags: '' } },
            misc: { unsortedTabs: 'top', uuid: 'made-up-install-id' },
            ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
          },
          status: { status: 'ok' },
        },
      },
    });
    await session.load();
    render(Settings, { section: 'categories' });

    await waitFor(() => expect(document.querySelector('.org-main-tabs')).not.toBeNull());
    expect(screen.getByRole('tablist', { name: 'Settings' })).toBeInTheDocument();
    expect(document.querySelector('.m-settings-nav')).toBeNull();
    expect(document.querySelector('.m-settings')).toBeNull();
  });
});
