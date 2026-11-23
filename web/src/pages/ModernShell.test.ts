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
        ping: { auth: 999, adminRefresh: 60000, everyoneRefresh: 60000 },
      },
      status: { status: 'ok' },
    },
  };
}

/** Stands in for the browser's media queries: a wide screen or a narrow one. */
function screen_(options: { wide: boolean }) {
  vi.stubGlobal(
    'matchMedia',
    (query: string) =>
      ({
        media: query,
        matches: options.wide,
        addEventListener: () => {},
        removeEventListener: () => {},
      }) as unknown as MediaQueryList,
  );
}

const page = createRawSnippet(() => ({ render: () => '<p>The page</p>' }));

async function openShell(
  loggedIn = true,
  shown: number | undefined = undefined,
  changes: { tabs?: MenuTab[]; categories?: MenuCategory[]; search?: unknown } = {},
) {
  const answer = launch(loggedIn);
  const data = answer.data as Record<string, unknown>;
  data.tabs = changes.tabs ?? TABS;
  data.categories = changes.categories ?? CATEGORIES;
  if (changes.search) {
    (data.settings as { homepage: Record<string, unknown> }).homepage.search = changes.search;
  }
  const sent = stubApi({
    'GET launch': answer,
    'POST ping': (request) => ({
      data: Object.fromEntries(
        ((request.body as { list: string[] }).list ?? []).map((address) => [address, 12]),
      ),
    }),
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
    await openShell();

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
    await openShell(true, 2);

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
    await openShell();
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
    await openShell(false);

    expect(screen.getByRole('link', { name: 'Log in' })).toHaveAttribute('href', '#/login');
    expect(screen.queryByRole('button', { name: 'Look' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Menu of the user' })).not.toBeInTheDocument();
  });

  it('look_button_opens_the_picker', async () => {
    await openShell();

    // The button keeps the look of the bar when it opens the popover.
    expect(screen.getByRole('button', { name: 'Look' })).toHaveClass('org-header-link');
    await fireEvent.click(screen.getByRole('button', { name: 'Look' }));
    const picker = (await screen.findByRole('radio', { name: 'Modern' })).closest(
      '[data-slot="popover-content"]',
    ) as HTMLElement;
    expect(picker).not.toBeNull();
    expect(within(picker).getByRole('radio', { name: 'Modern' })).toBeChecked();
    expect(within(picker).getByRole('button', { name: 'Use the default' })).toBeInTheDocument();
  });

  it('user_menu_has_account_lock_and_logout', async () => {
    const sent = await openShell();
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
    const sent = await openShell();

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Menu of the user' }), {
      key: 'Enter',
    });
    const menu = await screen.findByRole('menu');
    await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Logout' }));

    await waitFor(() => expect(at(sent, sent.length - 1).path).toBe('launch'));
    expect(sent.map((one) => one.path)).toContain('logout');
  });

  it('has_the_frame_buttons_and_full_screen', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    await openShell(true, 2);

    await fireEvent.click(screen.getByRole('button', { name: 'Open in a tab of the browser' }));
    expect(open).toHaveBeenCalledWith('https://2.example.com/', '_blank', 'noopener');
    expect(screen.getByRole('button', { name: 'Load again' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close the tab' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Full screen' })).toBeInTheDocument();
  });

  it('close_the_tab_goes_to_the_start', async () => {
    await openShell(true, 2);
    window.location.hash = '#Plex-Server';

    await fireEvent.click(screen.getByRole('button', { name: 'Close the tab' }));

    expect(window.location.hash).toBe('#/');
  });

  it('shows_whether_the_applications_answer', async () => {
    const tabs = [tab(1, 'Status', { ping: 1, ping_url: 'https://1.example.com/' })];
    const sent = await openShell(true, undefined, { tabs, categories: [] });

    await waitFor(() => expect(screen.getByRole('img', { name: 'answers' })).toBeInTheDocument());
    expect(sent.find((one) => one.path === 'ping')?.body).toEqual({
      list: ['https://1.example.com/'],
    });
  });

  it('has_the_media_search_when_the_caller_has_it', async () => {
    await openShell(true, undefined, { search: { enabled: true, type: 'plex' } });

    expect(screen.getByRole('button', { name: /search/i })).toBeInTheDocument();
  });

  it('has_the_links_at_the_end_of_the_menu', async () => {
    await openShell();

    const extras = screen.getByRole('navigation', { name: 'Organizr' });
    expect(within(extras).getByRole('button', { name: 'Logout' })).toBeInTheDocument();
    expect(within(extras).getByRole('link', { name: 'Organizr Docs' })).toHaveAttribute(
      'href',
      'https://docs.organizr.app',
    );
  });

  it('a_guest_finds_login_register_in_the_menu', async () => {
    await openShell(false);

    const extras = screen.getByRole('navigation', { name: 'Organizr' });
    expect(within(extras).getByRole('link', { name: 'Login/Register' })).toHaveAttribute(
      'href',
      '#/login',
    );
  });
});
