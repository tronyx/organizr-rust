import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { session } from '../lib/session.svelte';
import type { MenuCategory, MenuTab } from '../lib/types';
import { at, stubApi } from '../test-helpers';
import ModernShell from './ModernShell.svelte';

function tab(id: number, name: string, changes: Partial<MenuTab> = {}): MenuTab {
  return {
    id,
    order: id,
    category_id: 0,
    name,
    url: `https://${id}.example.com/`,
    url_local: '',
    default: 0,
    enabled: 1,
    group_id: 4,
    group_id_max: 0,
    add_to_admin: 0,
    image: null,
    type: 1,
    splash: 0,
    ping: 0,
    ping_url: null,
    timeout: 0,
    timeout_ms: null,
    preload: 0,
    access_url: `https://${id}.example.com/`,
    ...changes,
  };
}

const TABS = [
  tab(1, 'Status'),
  tab(2, 'Plex Server', { category_id: 7 }),
  tab(3, 'Sonarr', { category_id: 7 }),
];

const CATEGORIES: MenuCategory[] = [
  { id: 1, order: 1, category: 'Unsorted', category_id: 0, image: null, default: 1, count: 1 },
  { id: 2, order: 2, category: 'Media', category_id: 7, image: null, default: 0, count: 2 },
];

function launch(loggedIn: boolean) {
  return {
    data: {
      user: loggedIn
        ? {
            username: 'robin',
            group: 'Admin',
            groupID: 1,
            email: null,
            image: null,
            userID: 5,
            loggedin: true,
            locked: 0,
          }
        : { username: 'guest', group: 'Guest', groupID: 999, loggedin: false },
      look: { layout: 'modern', theme: 'organizr', colourSet: 'dark', source: 'user' },
      tabs: TABS,
      categories: CATEGORIES,
      plugins: { includes: {} },
      appearance: { title: 'My Organizr', useLogo: false },
      settings: {
        sso: { plex: { oAuthEnabled: false } },
        homepage: { refresh: {}, options: {} },
        misc: { checkForUpdate: false },
        ping: {},
      },
      status: { status: 'ok' },
    },
  };
}

/** Stands in for the browser's media queries: a wide screen or a narrow one, and the motion. */
function screen_(options: { wide: boolean; reducedMotion?: boolean }) {
  vi.stubGlobal(
    'matchMedia',
    (query: string) =>
      ({
        media: query,
        matches: query.includes('prefers-reduced-motion')
          ? (options.reducedMotion ?? false)
          : options.wide,
        addEventListener: () => {},
        removeEventListener: () => {},
      }) as unknown as MediaQueryList,
  );
}

const page = createRawSnippet(() => ({ render: () => '<p>The page</p>' }));

async function open(loggedIn = true, shown: number | undefined = undefined) {
  const sent = stubApi({
    'GET launch': launch(loggedIn),
    'POST users/lock': { message: 'locked' },
    'GET logout': { message: 'bye' },
  });
  await session.load();
  render(ModernShell, { shown, children: page });
  return sent;
}

describe('ModernShell', () => {
  beforeEach(() => screen_({ wide: true }));
  afterEach(() => {
    delete document.documentElement.dataset.layout;
  });

  it('tabs_are_grouped_by_category', async () => {
    await open();

    const menu = screen.getByRole('navigation', { name: 'Tabs' });
    const media = within(menu).getByText('Media').closest('details');
    expect(media).not.toBeNull();
    const links = within(media as HTMLElement)
      .getAllByRole('link')
      .map((link) => link.getAttribute('aria-label'));
    expect(links).toEqual(['Plex Server', 'Sonarr']);
    // A tab without a category stands outside the groups.
    expect(within(menu).getByRole('link', { name: 'Status' }).closest('details')).toBeNull();
    // The shell reuses the classes that custom CSS styles.
    expect(menu).toHaveClass('org-menu');
    expect(document.querySelector('.org-sidebar')).not.toBeNull();
    expect(document.querySelector('.org-sidebar-head')).not.toBeNull();
    expect(document.querySelector('.org-header')).not.toBeNull();
    expect(document.querySelector('.org-logo')).not.toBeNull();
  });

  it('open_tab_is_marked_current', async () => {
    await open(true, 2);

    expect(screen.getByRole('link', { name: 'Plex Server' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('link', { name: 'Sonarr' })).not.toHaveAttribute('aria-current');
    // The name of the shown tab stands in the top bar, and the group of it is open.
    expect(within(screen.getByRole('banner')).getByText('Plex Server')).toBeInTheDocument();
    expect(screen.getByText('Media').closest('details')).toHaveAttribute('open');
  });

  it('menu_is_a_drawer_on_a_narrow_screen', async () => {
    screen_({ wide: false });
    await open();
    expect(screen.queryByRole('navigation', { name: 'Tabs' })).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Show the menu of tabs' }));
    const drawer = await screen.findByRole('dialog');
    expect(within(drawer).getByRole('navigation', { name: 'Tabs' })).toBeInTheDocument();

    // Choosing a tab closes the drawer.
    await fireEvent.click(within(drawer).getByRole('link', { name: 'Status' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

    // Escape closes it too.
    await fireEvent.click(screen.getByRole('button', { name: 'Show the menu of tabs' }));
    await screen.findByRole('dialog');
    await fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('guest_sees_log_in_and_no_look_button', async () => {
    await open(false);

    expect(screen.getByRole('link', { name: 'Log in' })).toHaveAttribute('href', '#/login');
    expect(screen.queryByRole('button', { name: 'Look' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Menu of the user' })).not.toBeInTheDocument();
  });

  it('user_menu_has_account_lock_and_logout', async () => {
    const sent = await open();
    expect(screen.getByRole('button', { name: 'Look' })).toBeInTheDocument();

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Menu of the user' }), {
      key: 'Enter',
    });
    const menu = await screen.findByRole('menu');
    expect(within(menu).getByRole('menuitem', { name: 'Account Settings' })).toHaveAttribute(
      'href',
      '#/account',
    );
    expect(within(menu).getByRole('menuitem', { name: 'Settings' })).toBeInTheDocument();

    await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Lock Screen' }));
    await waitFor(() => expect(sent.map((one) => one.path)).toContain('users/lock'));
  });

  it('logout_asks_the_server_to_log_out', async () => {
    const sent = await open();

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Menu of the user' }), {
      key: 'Enter',
    });
    const menu = await screen.findByRole('menu');
    await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Logout' }));

    await waitFor(() => expect(at(sent, sent.length - 1).path).toBe('launch'));
    expect(sent.map((one) => one.path)).toContain('logout');
  });

  it('motion_is_off_when_reduced_motion_is_asked', async () => {
    screen_({ wide: true, reducedMotion: true });
    await open();
    expect(document.querySelector('[data-motion]')).toHaveAttribute('data-motion', 'off');
  });

  it('motion_is_on_otherwise', async () => {
    await open();
    expect(document.querySelector('[data-motion]')).toHaveAttribute('data-motion', 'on');
  });
});
