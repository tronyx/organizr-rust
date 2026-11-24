import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it } from 'vitest';

import { session } from '../lib/session.svelte';
import { stubApi } from '../test-helpers';
import Shell from './Shell.svelte';

const page = createRawSnippet(() => ({ render: () => '<p>The page</p>' }));

describe('Shell', () => {
  it('the_user_menu_look_entry_opens_the_picker', async () => {
    stubApi({
      'GET launch': {
        data: {
          user: {
            username: 'robin',
            group: 'Admin',
            groupID: 1,
            email: null,
            image: null,
            userID: 5,
            loggedin: true,
            locked: 0,
          },
          look: { layout: 'og', theme: 'Organizr', colourSet: 'dark', source: 'default' },
          tabs: [],
          categories: [],
          plugins: { includes: {} },
          appearance: { title: 'My Organizr', useLogo: false },
          settings: {
            sso: { plex: { oAuthEnabled: false } },
            homepage: { refresh: {}, options: {} },
            misc: { checkForUpdate: false },
            ping: { auth: 999, adminRefresh: 60000, everyoneRefresh: 60000 },
          },
          status: { status: 'ok' },
        },
      },
    });
    await session.load();
    render(Shell, { shown: undefined, children: page });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    const menu = screen.getByLabelText('Menu of the user').closest('details');
    await fireEvent.click(within(menu as HTMLElement).getByRole('button', { name: 'Look' }));

    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByRole('radio', { name: 'OG Organizr' })).toBeChecked();
    expect(within(dialog).getByRole('group', { name: 'Theme' })).toBeInTheDocument();
  });
});
