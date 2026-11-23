import { render, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from './test-helpers';

// The chunk of the modern shell does not load, as after an update with a cached page.
vi.mock('./pages/ModernShell.svelte', () => {
  throw new Error('Failed to fetch dynamically imported module');
});

describe('App', () => {
  it('falls_back_to_the_original_shell_when_the_modern_chunk_does_not_load', async () => {
    stubApi({
      'GET launch': {
        data: {
          user: { username: 'guest', group: 'Guest', groupID: 999, loggedin: false },
          look: { layout: 'modern', theme: 'organizr', colourSet: 'dark', source: 'user' },
          tabs: [],
          categories: [],
          plugins: { includes: {} },
          appearance: { title: 'My Organizr' },
          settings: {
            sso: { plex: { oAuthEnabled: false } },
            homepage: { refresh: {}, options: {} },
            misc: {},
            ping: {},
          },
          status: { status: 'ok' },
        },
      },
    });
    const { default: App } = await import('./App.svelte');

    const { container } = render(App);

    // The original shell has the class `org-with-menu` on its bar.
    await waitFor(() =>
      expect(container.ownerDocument.querySelector('.org-header')).not.toBeNull(),
    );
    expect(document.querySelector('.org-header.org-with-menu')).not.toBeNull();
    expect(document.querySelector('.loading-spinner')).toBeNull();
  });
});
