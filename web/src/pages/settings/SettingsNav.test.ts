import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { PAGES } from '../../lib/route.svelte';
import { SETTINGS_MENU, settingsPlace } from '../../lib/settingsMenu';
import SettingsNav from './SettingsNav.svelte';

/** Stands in for the browser's media queries: a wide screen or a narrow one. */
function screenSize(wide: boolean) {
  vi.stubGlobal(
    'matchMedia',
    (query: string) =>
      ({
        media: query,
        matches: wide,
        addEventListener: () => {},
        removeEventListener: () => {},
      }) as unknown as MediaQueryList,
  );
}

afterEach(() => vi.unstubAllGlobals());

describe('SettingsNav', () => {
  it('groups_and_pages_match_the_og_menu', () => {
    screenSize(true);
    render(SettingsNav, { section: 'tabs' });
    const nav = screen.getByRole('navigation', { name: 'Settings' });
    expect(nav).toHaveClass('m-settings-nav');

    const headings = within(nav)
      .getAllByRole('heading')
      .map((heading) => heading.textContent?.trim());
    expect(headings).toEqual(SETTINGS_MENU.map((group) => group.name));
    expect(headings).toEqual([
      'Tab Editor',
      'Customize',
      'User Management',
      'Image Manager',
      'Plugins',
      'System Settings',
    ]);

    const lists = within(nav).getAllByRole('list');
    expect(lists).toHaveLength(6);
    lists.forEach((list, index) => {
      const labels = within(list)
        .getAllByRole('link')
        .map((link) => link.textContent?.trim());
      expect(labels).toEqual(SETTINGS_MENU[index]?.pages.map((page) => page.label));
    });
    expect(within(nav).getAllByRole('link')).toHaveLength(22);
  });

  it('open_section_is_marked_current', () => {
    screenSize(true);
    render(SettingsNav, { section: 'logs' });
    const nav = screen.getByRole('navigation', { name: 'Settings' });
    const current = within(nav)
      .getAllByRole('link')
      .filter((link) => link.getAttribute('aria-current') === 'page');
    expect(current).toHaveLength(1);
    expect(current[0]).toHaveTextContent('Logs');
  });

  it('links_go_to_the_same_addresses_as_og', () => {
    screenSize(true);
    render(SettingsNav, { section: 'tabs' });
    const nav = screen.getByRole('navigation', { name: 'Settings' });
    const hrefs = within(nav)
      .getAllByRole('link')
      .map((link) => link.getAttribute('href'));
    expect(hrefs).toEqual(SETTINGS_MENU.flatMap((group) => group.pages.map((p) => PAGES[p.page])));
    expect(settingsPlace('plugin-marketplace').page.page).toBe('pluginMarketplace');
  });

  it('settings_menu_is_a_button_on_a_narrow_screen', async () => {
    screenSize(false);
    render(SettingsNav, { section: 'groups' });
    expect(screen.queryByRole('navigation', { name: 'Settings' })).not.toBeInTheDocument();

    const button = screen.getByRole('button', { name: /Settings menu/ });
    // The button names the group and the page that are open.
    expect(button).toHaveTextContent('User Management');
    expect(button).toHaveTextContent('Manage Groups');

    await fireEvent.click(button);
    const sheet = await screen.findByRole('dialog');
    const nav = within(sheet).getByRole('navigation', { name: 'Settings' });
    expect(within(nav).getAllByRole('link')).toHaveLength(22);

    await fireEvent.click(within(nav).getByRole('link', { name: 'Logs' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });
});
