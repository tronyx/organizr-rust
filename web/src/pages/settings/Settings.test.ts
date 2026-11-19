import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import type { EditorData, Tab } from '../../lib/types';
import { stubApi, type CannedAnswer, type SentRequest } from '../../test-helpers';
import Settings from './Settings.svelte';

function tab(id: number, name: string, changes: Partial<Tab> = {}): Tab {
  return {
    id,
    order: id,
    category_id: 0,
    name,
    url: `https://${name.toLowerCase()}.example.com/`,
    url_local: null,
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
    ...changes,
  };
}

const DATA: EditorData = {
  tabs: [
    tab(3, 'Plex', { category_id: 7, order: 5 }),
    tab(1, 'Settings', { type: 0, url: 'api/v2/page/settings', group_id: 1, order: 2 }),
    tab(4, 'Sonarr', { order: 9, enabled: 0, group_id: 999, group_id_max: 999 }),
  ],
  categories: [
    { id: 1, order: 1, category: 'Unsorted', category_id: 0, image: null, default: 1 },
    { id: 2, order: 2, category: 'Media', category_id: 7, image: null, default: 0 },
  ],
  groups: [
    { id: 1, group: 'Admin', group_id: 0, image: null, default: 0 },
    { id: 2, group: 'Co-Admin', group_id: 1, image: null, default: 0 },
    { id: 5, group: 'User', group_id: 4, image: null, default: 1 },
    { id: 6, group: 'Guest', group_id: 999, image: null, default: 0 },
  ],
};

const LAUNCH: CannedAnswer = {
  data: {
    categories: [],
    tabs: [],
    user: null,
    version: '0.1.0',
    appearance: { title: 'Organizr' },
    settings: { misc: { unsortedTabs: 'top' } },
    status: { status: 'ok' },
  },
};

type Answers = Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>;

/** Shows the settings on a stand-in for the API, and waits for the table. */
async function show(section: 'tabs' | 'categories' | 'groups', answers: Answers = {}) {
  const sent = stubApi({ 'GET tabs': { data: DATA }, 'GET launch': LAUNCH, ...answers });
  render(Settings, { section });
  await screen.findByRole('table');
  return sent;
}

/** The requests that changed something. */
function changes(sent: SentRequest[]) {
  return sent
    .filter((request) => request.method !== 'GET')
    .map((request) => [`${request.method} ${request.path}`, request.body]);
}

/** Waits until the line that reports the last change holds the text. */
async function statusSays(text: string) {
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent(text));
}

function rowOf(name: string): HTMLElement {
  const row = screen.getByRole('rowheader', { name: new RegExp(name) }).closest('tr');
  if (!row) {
    throw new Error(`The table has no row for ${name}.`);
  }
  return row;
}

describe('Settings, tabs', () => {
  it('lists the tabs in the order of the menu, with their category and groups', async () => {
    await show('tabs');

    const rows = screen.getAllByRole('row').slice(1);
    expect(rows.map((row) => within(row).getByRole('rowheader').textContent?.trim())).toEqual([
      'S Settings',
      'P Plex',
      'S Sonarr',
    ]);
    expect(within(rowOf('Plex')).getByText('Media')).toBeInTheDocument();
    expect(within(rowOf('Plex')).getByText('Admin to User')).toBeInTheDocument();
    expect(within(rowOf('Sonarr')).getByText('Guest')).toBeInTheDocument();
    expect(within(rowOf('Settings')).getByText('Page of Organizr')).toBeInTheDocument();
  });

  it('takes a tab out of the menu and loads the table and the menu again', async () => {
    const sent = await show('tabs', { 'PUT tabs/3': { message: 'Tab info updated' } });

    await fireEvent.click(screen.getByLabelText('Show Plex in the menu'));

    await statusSays('The tab is changed.');
    expect(changes(sent)).toEqual([['PUT tabs/3', { enabled: 0 }]]);
    expect(
      sent
        .slice(-2)
        .map((request) => request.path)
        .sort(),
    ).toEqual(['launch', 'tabs']);
  });

  it('makes a tab the one that opens at the start', async () => {
    const sent = await show('tabs', { 'PUT tabs/4': {} });

    await fireEvent.click(screen.getByLabelText('Open Sonarr at the start'));

    await waitFor(() => expect(changes(sent)).toEqual([['PUT tabs/4', { default: 1 }]]));
  });

  it('moves a tab and sends every place, numbered from 1', async () => {
    const sent = await show('tabs', { 'PUT tabs': {} });
    expect(screen.getByLabelText('Move Settings up')).toBeDisabled();
    expect(screen.getByLabelText('Move Sonarr down')).toBeDisabled();

    await fireEvent.click(screen.getByLabelText('Move Sonarr up'));

    await waitFor(() =>
      expect(changes(sent)).toEqual([
        [
          'PUT tabs',
          [
            { id: 1, order: 1 },
            { id: 4, order: 2 },
            { id: 3, order: 3 },
          ],
        ],
      ]),
    );
  });

  it('deletes a tab only after the user said yes', async () => {
    const sent = await show('tabs', { 'DELETE tabs/3': { status: 204 } });
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);

    await fireEvent.click(within(rowOf('Plex')).getByRole('button', { name: /Delete/ }));
    expect(confirm).toHaveBeenCalledWith('Delete the tab Plex?');
    expect(changes(sent)).toEqual([]);

    confirm.mockReturnValue(true);
    await fireEvent.click(within(rowOf('Plex')).getByRole('button', { name: /Delete/ }));
    await statusSays('The tab is deleted.');
    expect(changes(sent)).toEqual([['DELETE tabs/3', undefined]]);
  });

  it('adds a tab with the values of the form', async () => {
    const sent = await show('tabs', { 'POST tabs': { message: 'Tab added' } });

    await fireEvent.click(screen.getByRole('button', { name: 'Add a tab' }));
    const dialog = await screen.findByRole('dialog', { name: 'Add a tab' });
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'Radarr' } });
    await fireEvent.input(within(dialog).getByLabelText('Address'), {
      target: { value: 'https://radarr.example.com/' },
    });
    await fireEvent.change(within(dialog).getByLabelText('Category'), { target: { value: '7' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await statusSays('The tab is added.');
    expect(changes(sent)).toEqual([
      [
        'POST tabs',
        {
          name: 'Radarr',
          url: 'https://radarr.example.com/',
          url_local: '',
          type: 1,
          image: '',
          category_id: 7,
          // The default group, and the Admin as the group with the most rights.
          group_id: 4,
          group_id_max: 0,
          enabled: 1,
          default: 0,
          add_to_admin: 0,
          preload: 0,
        },
      ],
    ]);
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('changes a tab from its stored values, and shows a refusal inside the form', async () => {
    const stored = tab(3, 'Plex', {
      category_id: 7,
      url: 'https://{domain}/plex',
      url_local: 'http://10.0.0.6/',
      add_to_admin: 1,
    });
    const sent = await show('tabs', {
      'GET tabs/3': { data: stored },
      'PUT tabs/3': { status: 409, message: 'Tab name: Sonarr is already taken' },
    });

    await fireEvent.click(within(rowOf('Plex')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Change the tab Plex' });
    // The form holds the address as it is stored, with its placeholder.
    expect(within(dialog).getByLabelText('Address')).toHaveValue('https://{domain}/plex');
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'Sonarr' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Tab name: Sonarr is already taken',
    );
    expect(changes(sent)).toEqual([
      [
        'PUT tabs/3',
        {
          name: 'Sonarr',
          url: 'https://{domain}/plex',
          url_local: 'http://10.0.0.6/',
          type: 1,
          image: '',
          category_id: 7,
          group_id: 4,
          group_id_max: 0,
          enabled: 1,
          default: 0,
          add_to_admin: 1,
          preload: 0,
        },
      ],
    ]);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('leaves the address and the kind of a page of Organizr itself alone', async () => {
    const stored = DATA.tabs[1] as Tab;
    const sent = await show('tabs', { 'GET tabs/1': { data: stored }, 'PUT tabs/1': {} });

    await fireEvent.click(within(rowOf('Settings')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).queryByLabelText('Address')).not.toBeInTheDocument();
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(changes(sent)).toHaveLength(1));
    const body = changes(sent)[0]?.[1];
    expect(body).not.toHaveProperty('url');
    expect(body).not.toHaveProperty('type');
    expect(body).toMatchObject({ name: 'Settings', group_id: 1 });
  });
});

describe('Settings, categories', () => {
  it('lists the categories with the count of their tabs', async () => {
    await show('categories');

    expect(within(rowOf('Unsorted')).getByRole('cell', { name: '2' })).toBeInTheDocument();
    expect(within(rowOf('Media')).getByRole('cell', { name: '1' })).toBeInTheDocument();
    expect(screen.getByLabelText('Put new tabs into Unsorted')).toBeChecked();
  });

  it('adds a category', async () => {
    const sent = await show('categories', { 'POST categories': {} });

    await fireEvent.click(screen.getByRole('button', { name: 'Add a category' }));
    const dialog = await screen.findByRole('dialog', { name: 'Add a category' });
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'Books' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await statusSays('The category is added.');
    expect(changes(sent)).toEqual([['POST categories', { category: 'Books', image: '' }]]);
  });

  it('changes the name, the order and the category for new tabs', async () => {
    const sent = await show('categories', { 'PUT categories/2': {}, 'PUT categories': {} });

    await fireEvent.click(within(rowOf('Media')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Change the category Media' });
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'Films' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(changes(sent)).toHaveLength(1));

    await fireEvent.click(screen.getByLabelText('Put new tabs into Media'));
    await waitFor(() => expect(changes(sent)).toHaveLength(2));
    await fireEvent.click(screen.getByLabelText('Move Media up'));
    await waitFor(() => expect(changes(sent)).toHaveLength(3));

    expect(changes(sent)).toEqual([
      ['PUT categories/2', { category: 'Films', image: '' }],
      ['PUT categories/2', { default: 1 }],
      [
        'PUT categories',
        [
          { id: 2, order: 1 },
          { id: 1, order: 2 },
        ],
      ],
    ]);
  });

  it('shows why the server did not delete a category', async () => {
    const sent = await show('categories', {
      'DELETE categories/2': { status: 409, message: 'Category name: Media still has tabs' },
    });
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    await fireEvent.click(within(rowOf('Media')).getByRole('button', { name: /Delete/ }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Category name: Media still has tabs',
    );
    expect(screen.getByRole('status').textContent).toBe('');
    expect(changes(sent)).toEqual([['DELETE categories/2', undefined]]);
  });
});

describe('Settings, groups', () => {
  it('lists the groups with their level, and marks the group for new users', async () => {
    await show('groups');

    const rows = screen.getAllByRole('row').slice(1);
    expect(rows.map((row) => within(row).getByRole('rowheader').textContent?.trim())).toEqual([
      'A Admin',
      'C Co-Admin',
      'U User',
      'G Guest',
    ]);
    expect(within(rowOf('Guest')).getByRole('cell', { name: '999' })).toBeInTheDocument();
    expect(screen.getByLabelText('Put new users into User')).toBeChecked();
  });

  it('does not offer to delete the fixed groups or to put new users into them', async () => {
    await show('groups');

    for (const name of ['Admin', 'Guest']) {
      // The whole name, because "Admin" is also a part of "Co-Admin".
      const row = rowOf(`^${name}$`);
      expect(within(row).getByRole('button', { name: /Delete/ })).toBeDisabled();
      expect(screen.getByLabelText(`Put new users into ${name}`)).toBeDisabled();
      expect(within(row).getByRole('button', { name: /Change/ })).toBeEnabled();
    }
    expect(within(rowOf('Co-Admin')).getByRole('button', { name: /Delete/ })).toBeEnabled();
    expect(screen.getByLabelText('Put new users into Co-Admin')).toBeEnabled();
  });

  it('adds a group', async () => {
    const sent = await show('groups', { 'POST groups': { message: 'Group added' } });

    await fireEvent.click(screen.getByRole('button', { name: 'Add a group' }));
    const dialog = await screen.findByRole('dialog', { name: 'Add a group' });
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'Editors' } });
    await fireEvent.input(within(dialog).getByLabelText('Picture'), {
      target: { value: 'https://example.com/editors.png' },
    });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await statusSays('The group is added.');
    expect(changes(sent)).toEqual([
      ['POST groups', { group: 'Editors', image: 'https://example.com/editors.png' }],
    ]);
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('changes a group, and shows a refusal inside the form', async () => {
    const sent = await show('groups', {
      'PUT groups/2': { status: 409, message: 'Group name: User is already taken' },
    });

    await fireEvent.click(within(rowOf('Co-Admin')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Change the group Co-Admin' });
    expect(within(dialog).getByLabelText('Name')).toHaveValue('Co-Admin');
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'User' } });
    await fireEvent.input(within(dialog).getByLabelText('Picture'), { target: { value: 'c.png' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Group name: User is already taken',
    );
    expect(changes(sent)).toEqual([['PUT groups/2', { group: 'User', image: 'c.png' }]]);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('puts new users into another group', async () => {
    const sent = await show('groups', { 'PUT groups/2': { message: 'Group info updated' } });

    await fireEvent.click(screen.getByLabelText('Put new users into Co-Admin'));

    await statusSays('The group is changed.');
    expect(changes(sent)).toEqual([['PUT groups/2', { default: 1 }]]);
  });

  it('deletes a group only after the user said yes, and shows a refusal', async () => {
    const sent = await show('groups', {
      'DELETE groups/5': {
        status: 409,
        message: 'Cannot delete group as group still has users assigned to it',
      },
    });
    const ask = vi.spyOn(window, 'confirm').mockReturnValueOnce(false).mockReturnValueOnce(true);

    await fireEvent.click(within(rowOf('User')).getByRole('button', { name: /Delete/ }));
    expect(changes(sent)).toEqual([]);
    await fireEvent.click(within(rowOf('User')).getByRole('button', { name: /Delete/ }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Cannot delete group as group still has users assigned to it',
    );
    expect(ask).toHaveBeenCalledWith('Delete the group User?');
    expect(changes(sent)).toEqual([['DELETE groups/5', undefined]]);
  });
});

describe('Settings', () => {
  it('says so when the data cannot be loaded', async () => {
    stubApi({ 'GET tabs': { status: 401, message: 'Not Authorized' } });
    render(Settings, { section: 'tabs' });

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
  });
});
