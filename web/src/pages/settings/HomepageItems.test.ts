import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { HOMEPAGE_ITEMS } from '../../settings/forms';
import { stubApi, type CannedAnswer, type SentRequest } from '../../test-helpers';
import HomepageItems from './HomepageItems.svelte';

const GROUPS = [
  { id: 1, group: 'Admin', group_id: 0, image: null, default: 0 },
  { id: 5, group: 'User', group_id: 4, image: null, default: 1 },
];

type Answers = Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>;

/** Shows the cards with the settings of `config`, and waits for them. */
async function show(config: Record<string, unknown>, answers: Answers = {}) {
  const sent = stubApi({
    'GET config': { data: config },
    'GET groups': { data: GROUPS },
    ...answers,
  });
  const ondone = vi.fn();
  render(HomepageItems, { ondone });
  await screen.findByRole('list');
  return { sent, ondone };
}

/** The names on the cards, in their order. */
function cardNames(): string[] {
  return within(screen.getByRole('list'))
    .getAllByRole('heading', { level: 3 })
    .map((heading) => heading.textContent ?? '');
}

/** The card of an item. */
function card(name: string): HTMLElement {
  const found = screen.getByRole('heading', { level: 3, name }).closest('li');
  if (!found) {
    throw new Error(`There is no card of ${name}`);
  }
  return found;
}

/** Opens the form of an item and waits for its Save button. */
async function open(name: string): Promise<HTMLElement> {
  await fireEvent.click(screen.getByRole('button', { name: `Settings of ${name}` }));
  const dialog = await screen.findByRole('dialog', { name });
  await within(dialog).findByRole('button', { name: 'Save' });
  return dialog;
}

describe('HomepageItems', () => {
  it('lists_every_item_of_the_license_with_its_picture_and_category_in_order', async () => {
    const { sent } = await show({ license: 'personal' });

    // Every item of the personal license is listed, the ones without a switch too.
    expect(cardNames()).toEqual(HOMEPAGE_ITEMS.map((item) => item.name));
    expect(cardNames()).toContain('CustomHTML');
    expect(cardNames()).toContain('Misc');
    const sonarr = card('Sonarr');
    expect(sonarr.querySelector('img')).toHaveAttribute('src', 'plugins/images/tabs/sonarr.png');
    expect(within(sonarr).getByText('PVR')).toBeInTheDocument();
    // The list comes from our own definitions, not from the old form of the server.
    expect(sent.some((request) => request.path.startsWith('settings/'))).toBe(false);
  });

  it('leaves_out_the_items_that_the_license_does_not_have', async () => {
    await show({ license: 'business' });

    expect(cardNames()).not.toContain('Sonarr');
    expect(cardNames()).not.toContain('Plex');
    // An item for every license and one for both licenses stay.
    expect(cardNames()).toContain('Bookmarks');
    expect(cardNames()).toContain('CustomHTML');
    expect(cardNames()).toEqual(
      HOMEPAGE_ITEMS.filter((item) => item.license?.includes('business') ?? true).map(
        (item) => item.name,
      ),
    );
  });

  it('opens_an_item_with_its_sections_in_the_order_of_its_definition', async () => {
    await show({ license: 'personal' });
    const dialog = await open('Tautulli');

    expect(
      within(dialog)
        .getAllByRole('tab')
        .map((tab) => tab.textContent?.trim()),
    ).toEqual([
      'Enable',
      'Connection',
      'Library stats',
      'Viewing stats',
      'Other stats',
      'Options',
      'API SOCKS',
    ]);
  });

  it('every_item_has_enable_and_connection_first_and_options_socks_about_last', () => {
    const head = ['Enable', 'Connection'];
    const tail = ['Options', 'API SOCKS', 'About'];
    for (const item of HOMEPAGE_ITEMS) {
      const titles = item.page.sections.map((section) => section.title);
      const first = head.filter((title) => titles.includes(title));
      const last = tail.filter((title) => titles.includes(title));
      expect(titles.slice(0, first.length), item.name).toEqual(first);
      expect(titles.slice(titles.length - last.length), item.name).toEqual(last);
      // The own sections of the item stand between them, and none of those comes twice.
      const own = titles.slice(first.length, titles.length - last.length);
      expect(
        own.filter((title) => head.includes(title) || tail.includes(title)),
        item.name,
      ).toEqual([]);
    }
  });

  it('a_test_connection_button_calls_the_test_route_and_shows_the_answer', async () => {
    const { sent } = await show(
      { license: 'personal' },
      { 'POST test/sonarr': { message: 'API Connection succeeded' } },
    );
    const dialog = await open('Sonarr');
    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Connection' }));
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Test Connection' }));

    await waitFor(() =>
      expect(within(dialog).getByRole('status')).toHaveTextContent('API Connection succeeded'),
    );
    expect(sent.filter((request) => request.method === 'POST').map((r) => r.path)).toEqual([
      'test/sonarr',
    ]);
  });

  it('save_sends_one_put_config_with_only_the_changes', async () => {
    const { sent, ondone } = await show(
      { license: 'personal', homepageSonarrEnabled: false, homepageSonarrAuth: '4' },
      { 'PUT config': {} },
    );
    const dialog = await open('Sonarr');
    await fireEvent.click(within(dialog).getByLabelText('Enable'));
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));
    expect(
      sent.filter((request) => request.method !== 'GET').map((r) => [r.method, r.path, r.body]),
    ).toEqual([['PUT', 'config', { homepageSonarrEnabled: true }]]);
  });

  it('says_so_when_the_settings_cannot_be_loaded', async () => {
    stubApi({ 'GET config': { status: 401, message: 'Not Authorized' } });
    render(HomepageItems, { ondone: vi.fn() });

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
  });
});
