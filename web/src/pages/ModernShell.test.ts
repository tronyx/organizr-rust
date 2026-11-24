import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { router } from '../lib/route.svelte';
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

function launch(loggedIn: boolean, groupID = 1) {
  return {
    data: {
      user: loggedIn
        ? {
            username: 'robin',
            group: 'Admin',
            groupID,
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
  changes: {
    tabs?: MenuTab[];
    categories?: MenuCategory[];
    search?: unknown;
    groupID?: number;
  } = {},
) {
  const answer = launch(loggedIn, changes.groupID);
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
    localStorage.clear();
    vi.restoreAllMocks();
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

  it('top_bar_names_an_internal_page', async () => {
    await openShell();
    const bar = screen.getByRole('banner');
    expect(within(bar).queryByText(/Settings ·/)).not.toBeInTheDocument();

    try {
      router.go('#/settings/tabs');
      await waitFor(() => expect(within(bar).getByText('Settings · Tabs')).toBeInTheDocument());
      router.go('#/settings/plugins/marketplace');
      await waitFor(() =>
        expect(within(bar).getByText('Settings · Plugins · Marketplace')).toBeInTheDocument(),
      );
      router.go('#/account');
      await waitFor(() => expect(within(bar).getByText('Account Settings')).toBeInTheDocument());
    } finally {
      router.go('#/');
    }
  });

  it('top_bar_names_only_a_page_that_the_person_sees', async () => {
    // Logged out: no title for the settings or the account.
    await openShell(false);
    const bar = screen.getByRole('banner');
    try {
      for (const address of ['#/settings/tabs', '#/account', '#/invites']) {
        router.go(address);
        await waitFor(() => expect(router.route.page).not.toBe('home'));
        expect(bar.querySelector('.m-shown')).toBeNull();
      }
    } finally {
      router.go('#/');
    }
  });

  it('top_bar_has_no_settings_title_for_a_person_who_cannot_manage', async () => {
    await openShell(true, undefined, { groupID: 4 });
    const bar = screen.getByRole('banner');
    try {
      router.go('#/settings/tabs');
      await waitFor(() => expect(router.route.page).toBe('settings'));
      expect(within(bar).queryByText(/Settings ·/)).not.toBeInTheDocument();
      // The account page is for every person who is logged in.
      router.go('#/account');
      await waitFor(() => expect(within(bar).getByText('Account Settings')).toBeInTheDocument());
    } finally {
      router.go('#/');
    }
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

  it('the_user_menu_text_is_as_large_as_the_og_menus', async () => {
    await openShell();

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Menu of the user' }), {
      key: 'Enter',
    });
    const menu = await screen.findByRole('menu');
    // The root font is 14 px: `text-base` is 14 px, `text-sm` 12.25 px. The sizes of
    // the browser are checked in the browser tests.
    for (const item of within(menu).getAllByRole('menuitem')) {
      expect(item).toHaveClass('text-base');
      expect(item).not.toHaveClass('text-sm');
    }
    const label = menu.querySelector('[data-slot="dropdown-menu-label"]');
    expect(label).toHaveClass('text-sm');
    expect(label).not.toHaveClass('text-xs');
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

  it('menu_collapses_to_icons_and_expands_again', async () => {
    await openShell(true, 2);
    const toggle = screen.getByRole('button', { name: 'Collapse the menu' });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(toggle).toHaveAttribute('aria-controls', 'm-menu');
    const aside = document.getElementById('m-menu');
    expect(aside?.tagName).toBe('ASIDE');
    expect(aside).not.toHaveClass('m-sidebar-icons');

    await fireEvent.click(toggle);
    const expand = screen.getByRole('button', { name: 'Expand the menu' });
    expect(expand).toHaveAttribute('aria-expanded', 'false');
    expect(aside).toHaveClass('m-sidebar-icons');
    expect(document.querySelector('.org-content')).toHaveClass('m-with-menu', 'm-with-icons');

    // Every tab keeps its name and shows its picture; the categories have no heading.
    const menu = screen.getByRole('navigation', { name: 'Tabs' });
    const links = within(menu).getAllByRole('link');
    expect(links.map((link) => link.getAttribute('aria-label'))).toEqual([
      'Status',
      'Plex Server',
      'Sonarr',
    ]);
    for (const link of links) {
      expect(link.textContent?.trim()).toBe(link.getAttribute('aria-label')?.charAt(0));
      expect(link.querySelector('[aria-hidden="true"]')).not.toBeNull();
    }
    expect(within(menu).getByRole('link', { name: 'Plex Server' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(menu.querySelector('details, summary')).toBeNull();
    expect(within(menu).queryByText('Media')).not.toBeInTheDocument();
    expect(document.querySelector('.org-sidebar-head')).toBeNull();
    // The links at the end are icons too, with their names.
    const extras = screen.getByRole('navigation', { name: 'Organizr' });
    expect(within(extras).getByRole('button', { name: 'Logout' }).textContent?.trim()).toBe('');
    expect(within(extras).getByRole('link', { name: 'Organizr Docs' })).toBeInTheDocument();

    await fireEvent.click(expand);
    expect(screen.getByRole('button', { name: 'Collapse the menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    expect(aside).not.toHaveClass('m-sidebar-icons');
    expect(within(menu).getByText('Media').closest('details')).not.toBeNull();
  });

  it('a_collapsed_tab_shows_its_name_as_a_tooltip', async () => {
    // jsdom has no ResizeObserver, which the placing of a tooltip uses.
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    localStorage.setItem('organizr.modernMenuCollapsed', 'true');
    await openShell();

    const link = screen.getByRole('link', { name: 'Sonarr' });
    // The pointer rests on the tab. (The keyboard focus is checked in the browser: the
    // tooltip waits for `:focus-visible`, which jsdom does not have.)
    await fireEvent.pointerEnter(link, { pointerType: 'mouse' });
    await fireEvent.pointerMove(link, { pointerType: 'mouse' });
    const tip = await waitFor(() => {
      const found = document.querySelector('[data-slot="tooltip-content"]');
      expect(found).not.toBeNull();
      return found as HTMLElement;
    });
    expect(tip).toHaveTextContent('Sonarr');
    expect(tip).toHaveClass('text-sm');
    // The link has its name once: the tooltip does not describe it with the same text.
    expect(link).toHaveAccessibleName('Sonarr');
    expect(link).not.toHaveAttribute('aria-describedby');
  });

  it('the_collapse_toggle_is_only_on_a_wide_screen', async () => {
    screen_({ wide: false });
    localStorage.setItem('organizr.modernMenuCollapsed', 'true');
    await openShell();

    expect(screen.queryByRole('button', { name: /(Collapse|Expand) the menu/ })).toBeNull();
    expect(screen.getByRole('button', { name: 'Show the menu of tabs' })).toBeInTheDocument();
    // The drawer shows the whole menu, with its headings.
    await fireEvent.click(screen.getByRole('button', { name: 'Show the menu of tabs' }));
    const drawer = await screen.findByRole('dialog');
    expect(within(drawer).getByText('Media')).toBeInTheDocument();
    expect(drawer).not.toHaveClass('m-sidebar-icons');
  });

  it('the_collapsed_menu_is_remembered_in_this_browser', async () => {
    localStorage.setItem('organizr.modernMenuCollapsed', 'true');
    await openShell();
    expect(screen.getByRole('button', { name: 'Expand the menu' })).toBeInTheDocument();
    expect(document.getElementById('m-menu')).toHaveClass('m-sidebar-icons');

    await fireEvent.click(screen.getByRole('button', { name: 'Expand the menu' }));
    expect(localStorage.getItem('organizr.modernMenuCollapsed')).toBe('false');
    await fireEvent.click(screen.getByRole('button', { name: 'Collapse the menu' }));
    expect(localStorage.getItem('organizr.modernMenuCollapsed')).toBe('true');
  });

  it('the_menu_follows_a_collapse_or_expand_in_another_tab_of_the_browser', async () => {
    await openShell();
    expect(document.getElementById('m-menu')).not.toHaveClass('m-sidebar-icons');

    // Another tab stores the choice: this one gets a `storage` event.
    const change = (newValue: string | null, key: string | null = 'organizr.modernMenuCollapsed') =>
      fireEvent(window, new StorageEvent('storage', { key, newValue }));
    await change('true');
    expect(document.getElementById('m-menu')).toHaveClass('m-sidebar-icons');
    expect(screen.getByRole('button', { name: 'Expand the menu' })).toBeInTheDocument();
    // Another key changes nothing.
    await change('false', 'organizr.other');
    expect(document.getElementById('m-menu')).toHaveClass('m-sidebar-icons');
    await change('false');
    expect(document.getElementById('m-menu')).not.toHaveClass('m-sidebar-icons');
    // A cleared storage opens the menu, as on a first visit.
    await change('true');
    await change(null, null);
    expect(document.getElementById('m-menu')).not.toHaveClass('m-sidebar-icons');
  });

  it('the_menu_starts_open_and_still_collapses_when_storage_is_blocked', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new DOMException('blocked', 'SecurityError');
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('blocked', 'SecurityError');
    });
    await openShell();

    await fireEvent.click(screen.getByRole('button', { name: 'Collapse the menu' }));
    expect(document.getElementById('m-menu')).toHaveClass('m-sidebar-icons');
    expect(screen.getByRole('link', { name: 'Status' })).toBeInTheDocument();
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
