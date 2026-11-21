import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { session } from '../../lib/session.svelte';
import type { Account, EditorData, Tab } from '../../lib/types';
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

const DATA: Omit<EditorData, 'users'> = {
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

function account(id: number, username: string, changes: Partial<Account> = {}): Account {
  return {
    id,
    username,
    email: `${username.toLowerCase()}@example.com`,
    group: 'User',
    group_id: 4,
    locked: 0,
    image: null,
    register_date: '2026-11-18 19:02:06',
    ...changes,
  };
}

const USERS: Account[] = [
  account(1, 'admin', { group: 'Admin', group_id: 0 }),
  account(3, 'zoe'),
  account(4, 'Yan', { locked: 1, register_date: '2026-12-01 08:00:00' }),
];

const LAUNCH: CannedAnswer = {
  data: {
    categories: [],
    tabs: [],
    user: null,
    version: '0.1.0',
    appearance: { title: 'Organizr' },
    plugins: {},
    settings: {
      sso: { plex: { oAuthEnabled: false } },
      homepage: { refresh: {}, options: { healthChecksTags: '' } },
      misc: { unsortedTabs: 'top', uuid: 'made-up-install-id' },
      ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
    },
    status: { status: 'ok' },
  },
};

type Answers = Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>;

/** Shows the settings on a stand-in for the API, and waits for the table. */
async function show(
  section: 'tabs' | 'categories' | 'groups' | 'users' | 'import',
  answers: Answers = {},
) {
  const sent = stubApi({
    'GET tabs': { data: DATA },
    'GET users': { data: USERS },
    'GET launch': LAUNCH,
    ...answers,
  });
  render(Settings, { section });
  if (section === 'import') {
    await screen.findByRole('heading', { name: 'Import Users' });
  } else {
    await screen.findByRole('table');
  }
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
      'Settings',
      'Plex',
      'Sonarr',
    ]);
    const chosen = (label: string) =>
      (screen.getByLabelText(label) as HTMLSelectElement).selectedOptions[0]?.textContent;
    expect(chosen('Category of Plex')).toBe('Media');
    expect(chosen('Least group that sees Plex')).toBe('User');
    expect(chosen('Most group that sees Plex')).toBe('Admin');
    expect(chosen('Least group that sees Sonarr')).toBe('Guest');
    expect(chosen('Type of Settings')).toBe('Organizr');
  });

  it('changes a field of a tab in its row, as PHP does', async () => {
    const sent = await show('tabs', { 'PUT tabs/3': {} });

    await fireEvent.change(screen.getByLabelText('Type of Plex'), { target: { value: '2' } });
    await fireEvent.click(screen.getByLabelText('Ping Plex'));

    await waitFor(() =>
      expect(changes(sent)).toEqual([
        ['PUT tabs/3', { type: 2 }],
        ['PUT tabs/3', { ping: 1 }],
      ]),
    );
  });

  it('takes a tab out of the menu and loads the table and the menu again', async () => {
    const sent = await show('tabs', { 'PUT tabs/3': { message: 'Tab info updated' } });

    await fireEvent.click(screen.getByLabelText('Show Plex in the menu'));

    await statusSays('The tab is changed.');
    expect(changes(sent)).toEqual([['PUT tabs/3', { enabled: 0 }]]);
    // After the change: the two lists of the settings, and the menu.
    expect(
      sent
        .slice(-3)
        .map((request) => request.path)
        .sort(),
    ).toEqual(['launch', 'tabs', 'users']);
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
    // The address to check shows only with the check switched on.
    expect(within(dialog).queryByLabelText('Address to check')).not.toBeInTheDocument();
    await fireEvent.click(within(dialog).getByLabelText('Checked whether it answers'));
    await fireEvent.input(within(dialog).getByLabelText('Address to check'), {
      target: { value: 'radarr.example.com:7878' },
    });
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
          ping: 1,
          ping_url: 'radarr.example.com:7878',
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
      ping_url: 'plex.example.com:32400',
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
          // The check is off. The stored address to check stays as it is.
          ping: 0,
          ping_url: 'plex.example.com:32400',
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
    expect(body).not.toHaveProperty('ping');
    expect(body).not.toHaveProperty('ping_url');
    expect(body).not.toHaveProperty('type');
    expect(body).toMatchObject({ name: 'Settings', group_id: 1 });
  });
});

describe('Settings, categories', () => {
  it('lists the categories with the count of their tabs', async () => {
    await show('categories');

    expect(within(rowOf('Unsorted')).getByRole('cell', { name: '2' })).toBeInTheDocument();
    expect(within(rowOf('Media')).getByRole('cell', { name: '1' })).toBeInTheDocument();
    expect(screen.getByLabelText('Put new tabs into Unsorted')).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByLabelText('Put new tabs into Media')).toHaveAttribute(
      'aria-pressed',
      'false',
    );
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
      'Admin',
      'Co-Admin',
      'User',
      'Guest',
    ]);
    expect(screen.getByLabelText('Put new users into User')).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByLabelText('Put new users into Co-Admin')).toHaveAttribute(
      'aria-pressed',
      'false',
    );
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

describe('Settings, users', () => {
  /** The launch answer for the Admin of the list, who looks at the page. */
  const AS_ADMIN: CannedAnswer = {
    data: {
      ...(LAUNCH.data as object),
      user: { username: 'admin', group: 'Admin', groupID: 0, userID: 1, loggedin: true },
    },
  };

  it('lists the users with their group and the day they were stored', async () => {
    await show('users');

    const rows = screen.getAllByRole('row').slice(1);
    expect(rows.map((row) => within(row).getByRole('rowheader').textContent?.trim())).toEqual([
      'admin',
      'zoe',
      'Yan',
    ]);
    const yan = rowOf('Yan');
    expect(within(yan).getByText('yan@example.com')).toBeInTheDocument();
    expect(within(yan).getByText('User')).toBeInTheDocument();
    expect(within(yan).getByText('2026-12-01')).toBeInTheDocument();
    expect(screen.getByLabelText('Lock Yan')).toBeChecked();
    expect(screen.getByLabelText('Lock zoe')).not.toBeChecked();
  });

  it('does not offer a person to lock or to delete the own account', async () => {
    stubApi({ 'GET launch': AS_ADMIN });
    await session.load();
    await show('users', { 'GET launch': AS_ADMIN });

    expect(screen.getByLabelText('Lock admin')).toBeDisabled();
    expect(within(rowOf('admin')).getByRole('button', { name: /Delete/ })).toBeDisabled();
    expect(within(rowOf('admin')).getByRole('button', { name: /Change/ })).toBeEnabled();
    expect(screen.getByLabelText('Lock zoe')).toBeEnabled();
    expect(within(rowOf('zoe')).getByRole('button', { name: /Delete/ })).toBeEnabled();
  });

  it('imports the users of a media server and says how many it added', async () => {
    const sent = await show('import', {
      'POST users/import/jellyfin': { message: 'Imported 2 users' },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Import Jellyfin users' }));

    await statusSays('The users of Jellyfin are imported.');
    expect(await screen.findByText('Imported 2 users')).toBeInTheDocument();
    expect(changes(sent)).toEqual([['POST users/import/jellyfin', { type: 'jellyfin' }]]);
    expect(screen.getByRole('button', { name: 'Import Plex users' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Import Emby users' })).toBeEnabled();
  });

  it('shows why an import failed', async () => {
    await show('import', {
      'POST users/import/plex': { status: 401, message: 'Not Authorized' },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Import Plex users' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
    expect(screen.queryByText(/Imported/)).not.toBeInTheDocument();
  });

  it('adds a user with a name, an address and a password', async () => {
    const sent = await show('users', { 'POST users': { message: 'User created' } });

    await fireEvent.click(screen.getByRole('button', { name: 'Add a user' }));
    const dialog = await screen.findByRole('dialog', { name: 'Add a user' });
    // The group and the picture of a user are set after the user exists.
    expect(within(dialog).queryByLabelText('Group')).not.toBeInTheDocument();
    const fill = (label: string, value: string) =>
      fireEvent.input(within(dialog).getByLabelText(label), { target: { value } });
    await fill('User name', 'Abe');
    await fill('E-mail address', 'abe@example.com');
    await fill('Password', 'the password of abe');
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await statusSays('The user is added.');
    expect(changes(sent)).toEqual([
      [
        'POST users',
        { username: 'Abe', email: 'abe@example.com', password: 'the password of abe' },
      ],
    ]);
  });

  it('sends only what the form changed', async () => {
    const sent = await show('users', { 'PUT users/3': { message: 'User info updated' } });

    await fireEvent.click(within(rowOf('zoe')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Change the user zoe' });
    expect(within(dialog).getByLabelText('User name')).toHaveValue('zoe');
    expect(within(dialog).getByLabelText('E-mail address')).toHaveValue('zoe@example.com');
    // Nobody is put into the group of guests.
    const group = within(dialog).getByLabelText('Group');
    expect(
      within(group)
        .getAllByRole('option')
        .map((option) => option.textContent),
    ).toEqual(['Admin', 'Co-Admin', 'User']);
    await fireEvent.change(group, { target: { value: '1' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await statusSays('The user is changed.');
    expect(changes(sent)).toEqual([['PUT users/3', { group_id: 1 }]]);
  });

  it('sends a new password only when one was entered, and shows a refusal', async () => {
    const sent = await show('users', {
      'PUT users/3': { status: 409, message: 'Username: Yan is already taken' },
    });

    await fireEvent.click(within(rowOf('zoe')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Change the user zoe' });
    await fireEvent.input(within(dialog).getByLabelText('User name'), { target: { value: 'Yan' } });
    await fireEvent.input(within(dialog).getByLabelText('New password'), {
      target: { value: 'a new password' },
    });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Username: Yan is already taken',
    );
    expect(changes(sent)).toEqual([
      ['PUT users/3', { username: 'Yan', password: 'a new password' }],
    ]);
  });

  it('closes the form without a request when nothing was changed', async () => {
    const sent = await show('users');

    await fireEvent.click(within(rowOf('zoe')).getByRole('button', { name: /Change/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Change the user zoe' });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(changes(sent)).toEqual([]);
  });

  it('locks and unlocks a user', async () => {
    const sent = await show('users', { 'PUT users/3': {}, 'PUT users/4': {} });

    await fireEvent.click(screen.getByLabelText('Lock zoe'));
    await statusSays('The user is locked.');
    await fireEvent.click(screen.getByLabelText('Lock Yan'));
    await statusSays('The user is unlocked.');

    expect(changes(sent)).toEqual([
      ['PUT users/3', { locked: 1 }],
      ['PUT users/4', { locked: 0 }],
    ]);
  });

  it('deletes a user only after the person said yes, and shows a refusal', async () => {
    const sent = await show('users', {
      'DELETE users/1': { status: 409, message: 'Cannot delete the last admin' },
    });
    const ask = vi.spyOn(window, 'confirm').mockReturnValueOnce(false).mockReturnValueOnce(true);

    await fireEvent.click(within(rowOf('admin')).getByRole('button', { name: /Delete/ }));
    expect(changes(sent)).toEqual([]);
    await fireEvent.click(within(rowOf('admin')).getByRole('button', { name: /Delete/ }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Cannot delete the last admin');
    expect(ask).toHaveBeenCalledWith('Delete the user admin?');
    expect(changes(sent)).toEqual([['DELETE users/1', undefined]]);
  });
});

describe('Settings, general', () => {
  const CONFIG = {
    title: 'Home',
    description: 'My server',
    logo: 'plugins/images/organizr/organizr-logo-h.png',
    useLogo: false,
    unsortedTabs: 'top',
    rememberMeDays: '7',
    loginAttempts: '5',
    loginLockout: '60000',
    localIPList: '',
    hideRegistration: false,
    registrationPassword: '***Secure***',
    organizrAPI: 'abcdefghij0123456789',
    plexToken: '',
  };

  /** Shows the general settings on a stand-in for the API, and waits for the form. */
  async function showGeneral(answers: Answers = {}) {
    const sent = stubApi({
      'GET tabs': { data: DATA },
      'GET users': { data: USERS },
      'GET launch': LAUNCH,
      'GET config': { data: CONFIG },
      'GET status': { data: { action: 'launch', database: 'postgresql' } },
      ...answers,
    });
    render(Settings, { section: 'general' });
    await screen.findByLabelText('Name of this install');
    return sent;
  }

  it('shows the stored values, and a secret only as stored or not stored', async () => {
    await showGeneral();

    expect(screen.getByLabelText('Name of this install')).toHaveValue('Home');
    expect(screen.getByLabelText(/Days that a login lasts/)).toHaveValue(7);
    expect(screen.getByLabelText(/Show the picture in the menu/)).not.toBeChecked();
    expect(screen.getByLabelText('Tabs without a heading stand')).toHaveValue('top');
    const registration = screen.getByLabelText('Registration password');
    expect(registration).toHaveValue('');
    expect(registration).toHaveAttribute('placeholder', 'A value is stored');
    expect(registration).toHaveAttribute('type', 'password');
    // The server shows the API key to an Admin.
    expect(screen.getByLabelText('API key')).toHaveAttribute('placeholder', 'abcdefghij0123456789');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('sends only the settings that the form changed', async () => {
    const sent = await showGeneral({ 'PUT config': { message: 'Config items updated' } });

    await fireEvent.input(screen.getByLabelText('Name of this install'), {
      target: { value: 'Media' },
    });
    await fireEvent.input(screen.getByLabelText(/Days that a login lasts/), {
      target: { value: '30' },
    });
    await fireEvent.click(screen.getByLabelText('Hide the link to make an account'));
    await fireEvent.change(screen.getByLabelText('Tabs without a heading stand'), {
      target: { value: 'bottom' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await statusSays('The settings are saved.');
    expect(changes(sent)).toEqual([
      [
        'PUT config',
        { title: 'Media', unsortedTabs: 'bottom', rememberMeDays: '30', hideRegistration: true },
      ],
    ]);
    // The form and the menu load their data again.
    expect(
      sent
        .slice(-2)
        .map((request) => request.path)
        .sort(),
    ).toEqual(['config', 'launch']);
  });

  it('sends a new registration password, and a new API key of 20 characters', async () => {
    const sent = await showGeneral({ 'PUT config': {} });

    await fireEvent.input(screen.getByLabelText('Registration password'), {
      target: { value: 'let them in' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Make a new key' }));
    const key = (screen.getByLabelText('API key') as HTMLInputElement).value;
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await statusSays('The settings are saved.');
    expect(key).toMatch(/^[A-Za-z0-9]{20}$/);
    expect(key).not.toBe(CONFIG.organizrAPI);
    expect(changes(sent)).toEqual([
      ['PUT config', { registrationPassword: 'let them in', organizrAPI: key }],
    ]);
  });

  it('shows why the server refused, and keeps what was entered', async () => {
    await showGeneral({
      'PUT config': { status: 422, message: 'Config item: rememberMeDays is not valid' },
    });

    await fireEvent.input(screen.getByLabelText(/Days that a login lasts/), {
      target: { value: '99999' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Config item: rememberMeDays is not valid',
    );
    expect(screen.getByLabelText(/Days that a login lasts/)).toHaveValue(99999);
    expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();
  });

  it('says so when the settings cannot be loaded', async () => {
    stubApi({
      'GET tabs': { data: DATA },
      'GET users': { data: USERS },
      'GET config': { status: 401, message: 'Not Authorized' },
      'GET status': { data: { action: 'launch', database: 'postgresql' } },
    });
    render(Settings, { section: 'general' });

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
  });
});

describe('Settings', () => {
  it('says so when the data cannot be loaded', async () => {
    const refused = { status: 401, message: 'Not Authorized' };
    stubApi({ 'GET tabs': refused, 'GET users': refused });
    render(Settings, { section: 'tabs' });

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
  });
});

describe('Settings, e-mail', () => {
  const CONFIG = {
    'PHPMAILER-enabled': false,
    'PHPMAILER-smtpHost': '',
    'PHPMAILER-smtpHostPort': '',
    'PHPMAILER-smtpHostType': 'tls',
    'PHPMAILER-verifyCert': true,
    'PHPMAILER-smtpHostAuth': true,
    'PHPMAILER-smtpHostUsername': '',
    'PHPMAILER-smtpHostPassword': '',
    'PHPMAILER-smtpHostSenderName': 'Organizr',
    'PHPMAILER-smtpHostSenderEmail': 'no-reply@Organizr.tld',
    'PHPMAILER-domain': '',
    'PHPMAILER-template': 'default',
    'PHPMAILER-logo': 'https://example.com/logo.png',
    'PHPMAILER-emailTemplateResetSubject': 'Password Reset',
    'PHPMAILER-emailTemplateReset': '<h2>Hey there {user}!</h2>',
    'PHPMAILER-emailTemplateRegisterUserEnabled': true,
    'PHPMAILER-emailTemplateRegisterUserSubject': 'Thank you For Registering',
    'PHPMAILER-emailTemplateRegisterUser': '<h2>Welcome {user}!</h2>',
  };

  async function showMail(answers: Answers = {}) {
    const sent = stubApi({
      'GET tabs': { data: DATA },
      'GET users': { data: USERS },
      'GET launch': LAUNCH,
      'GET config': { data: CONFIG },
      ...answers,
    });
    render(Settings, { section: 'mail' });
    await screen.findByLabelText('SMTP server');
    return sent;
  }

  it('saves the server, its password and the text of an e-mail', async () => {
    const sent = await showMail({ 'PUT config': {} });
    expect(screen.getByRole('tab', { name: 'PHP Mailer' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(screen.getByLabelText('Password')).toHaveAttribute('placeholder', 'No value is stored');
    const texts = screen.getAllByLabelText('Text, as HTML');
    expect(texts[0]).toHaveValue('<h2>Hey there {user}!</h2>');

    await fireEvent.click(screen.getByLabelText('Send e-mails'));
    await fireEvent.input(screen.getByLabelText('SMTP server'), {
      target: { value: 'smtp.example.com' },
    });
    await fireEvent.change(screen.getByLabelText('Encryption'), { target: { value: 'ssl' } });
    await fireEvent.input(screen.getByLabelText('Password'), {
      target: { value: 'the password of the mailbox' },
    });
    await fireEvent.input(texts[0] as HTMLElement, { target: { value: '<p>New: {password}</p>' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await statusSays('The settings are saved.');
    expect(changes(sent)).toEqual([
      [
        'PUT config',
        {
          'PHPMAILER-enabled': true,
          'PHPMAILER-smtpHost': 'smtp.example.com',
          'PHPMAILER-smtpHostType': 'ssl',
          'PHPMAILER-smtpHostPassword': 'the password of the mailbox',
          'PHPMAILER-emailTemplateReset': '<p>New: {password}</p>',
        },
      ],
    ]);
  });

  it('sends a test e-mail and says where it went, or why it was not sent', async () => {
    let works = false;
    const sent = await showMail({
      'GET plugins/php-mailer/email/test': () =>
        works
          ? { message: 'Email sent' }
          : { status: 500, message: 'the SMTP server did not take the e-mail: refused' },
    });
    const button = screen.getByRole('button', { name: 'Send a test e-mail' });

    await fireEvent.click(button);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The test e-mail was not sent: the SMTP server did not take the e-mail: refused',
    );

    works = true;
    await fireEvent.click(button);
    await statusSays('The test e-mail was sent to the address of your account.');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(sent.filter((request) => request.path.endsWith('email/test'))).toHaveLength(2);
  });
});

describe('Settings, login', () => {
  const CONFIG = {
    authType: 'internal',
    authBackend: '',
    plexToken: '',
    plexID: '',
    plexAdmin: '',
    plexStrictFriends: true,
    plexoAuth: false,
    ignoreTFAIfPlexOAuth: false,
    ignoreTFALocal: false,
    ssoPlex: false,
    ssoOmbi: false,
    ombiURL: '',
    ombiFallbackUser: '',
    ombiFallbackPassword: '',
    ssoOverseerr: false,
    overseerrURL: '',
    overseerrFallbackUser: '',
    overseerrFallbackPassword: '',
    ssoTautulli: false,
    tautulliURL: '',
    ssoTautulliAuth: '4',
    authBackendHost: '',
    authBaseDN: '',
    authBackendHostPrefix: '',
    authBackendHostSuffix: '',
    ldapBindUsername: '',
    ldapBindPassword: '',
    ldapType: '1',
    ldapSSL: false,
    ldapTLS: false,
    embyURL: '',
    embyToken: '',
    jellyfinURL: '',
    jellyfinToken: '',
  };

  async function showLogin(answers: Answers = {}, config: Record<string, unknown> = CONFIG) {
    const sent = stubApi({
      'GET tabs': { data: DATA },
      'GET users': { data: USERS },
      'GET launch': LAUNCH,
      'GET config': { data: config },
      ...answers,
    });
    render(Settings, { section: 'login' });
    await screen.findByLabelText('A password is checked by');
    return sent;
  }

  it('saves who checks a password, the Plex token and a single sign-on', async () => {
    const sent = await showLogin({ 'PUT config': {} });
    expect(screen.getByRole('tab', { name: 'Login' })).toHaveAttribute('aria-selected', 'true');

    await fireEvent.change(screen.getByLabelText('A password is checked by'), {
      target: { value: 'both' },
    });
    await fireEvent.change(screen.getByLabelText('Service'), { target: { value: 'plex' } });
    await fireEvent.input(screen.getByLabelText('Plex token of the owner of the server'), {
      target: { value: 'made-up-token' },
    });
    await fireEvent.click(screen.getByLabelText('Show the button "Log in with Plex"'));
    await fireEvent.click(screen.getByLabelText('Log the user in to Tautulli'));
    await fireEvent.input(screen.getByLabelText('Address of Tautulli'), {
      target: { value: 'https://tautulli.example.com' },
    });
    await fireEvent.change(screen.getByLabelText('For users of the level'), {
      target: { value: '1' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await statusSays('The settings are saved.');
    expect(changes(sent)).toEqual([
      [
        'PUT config',
        {
          authType: 'both',
          authBackend: 'plex',
          plexToken: 'made-up-token',
          plexoAuth: true,
          ssoTautulli: true,
          tautulliURL: 'https://tautulli.example.com',
          ssoTautulliAuth: '1',
        },
      ],
    ]);
  });

  it('shows the settings of the chosen back end only, and none when Organizr checks alone', async () => {
    await showLogin();
    const shown = (label: string) => screen.queryByLabelText(label) !== null;
    expect(shown('Address of the server')).toBe(false);
    expect(shown('Base DN')).toBe(false);

    // The back end alone does not show its settings while Organizr checks alone.
    await fireEvent.change(screen.getByLabelText('Service'), { target: { value: 'ldap' } });
    expect(shown('Base DN')).toBe(false);

    await fireEvent.change(screen.getByLabelText('A password is checked by'), {
      target: { value: 'both' },
    });
    expect(shown('Address of the server')).toBe(true);
    expect(shown('Base DN')).toBe(true);
    expect(shown('Bind password')).toBe(true);
    expect(shown('Address of Emby')).toBe(false);

    await fireEvent.change(screen.getByLabelText('Service'), { target: { value: 'ftp' } });
    expect(shown('Address of the server')).toBe(true);
    expect(shown('Base DN')).toBe(false);

    for (const emby of ['emby_local', 'emby_connect', 'emby_all']) {
      await fireEvent.change(screen.getByLabelText('Service'), { target: { value: emby } });
      expect(shown('Address of Emby')).toBe(true);
      expect(shown('API key of Emby')).toBe(true);
      expect(shown('Address of Jellyfin')).toBe(false);
      expect(shown('Address of the server')).toBe(false);
    }

    await fireEvent.change(screen.getByLabelText('Service'), { target: { value: 'jellyfin' } });
    expect(shown('Address of Jellyfin')).toBe(true);
    expect(shown('Address of Emby')).toBe(false);
  });

  it('saves the LDAP settings and shows the name of an account', async () => {
    const sent = await showLogin({ 'PUT config': {} });
    await fireEvent.change(screen.getByLabelText('A password is checked by'), {
      target: { value: 'external' },
    });
    await fireEvent.change(screen.getByLabelText('Service'), { target: { value: 'ldap' } });
    const type = (label: string, value: string) =>
      fireEvent.input(screen.getByLabelText(label), { target: { value } });
    await type('Address of the server', 'ldap://ldap.example.com');
    await type('Base DN', 'dc=example,dc=com');
    await type('Account prefix', 'uid=');
    await type('Account suffix', ',ou=people,dc=example,dc=com');
    await type('Bind user name', 'cn=organizr,dc=example,dc=com');
    await type('Bind password', 'made-up-bind-password');
    await fireEvent.change(screen.getByLabelText('Type of the LDAP server'), {
      target: { value: '2' },
    });
    await fireEvent.click(screen.getByLabelText('Turn the connection to TLS (StartTLS)'));
    expect(screen.getByText('uid=TestAcct,ou=people,dc=example,dc=com')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await statusSays('The settings are saved.');
    expect(changes(sent)).toEqual([
      [
        'PUT config',
        {
          authType: 'external',
          authBackend: 'ldap',
          authBackendHost: 'ldap://ldap.example.com',
          authBaseDN: 'dc=example,dc=com',
          authBackendHostPrefix: 'uid=',
          authBackendHostSuffix: ',ou=people,dc=example,dc=com',
          ldapBindUsername: 'cn=organizr,dc=example,dc=com',
          ldapBindPassword: 'made-up-bind-password',
          ldapType: '2',
          ldapTLS: true,
        },
      ],
    ]);
  });

  it('tests the LDAP connection, and a login with a name and a password', async () => {
    const sent = await showLogin(
      {
        'POST test/ldap': { message: 'LDAP connection successful' },
        'POST test/ldap/login': (request) =>
          (request.body as { password?: string }).password === 'right'
            ? { message: 'LDAP connection successful' }
            : { status: 401, message: 'Username/Password Failed to authenticate' },
      },
      { ...CONFIG, authType: 'both', authBackend: 'ldap', authBaseDN: 'dc=example,dc=com' },
    );

    await fireEvent.click(screen.getByRole('button', { name: 'Test the connection of LDAP' }));
    expect(await screen.findByText('LDAP connection successful')).toBeInTheDocument();

    await fireEvent.input(screen.getByLabelText('User name for the test'), {
      target: { value: 'robin' },
    });
    await fireEvent.input(screen.getByLabelText('Password for the test'), {
      target: { value: 'wrong' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test a login' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Username/Password Failed to authenticate',
    );

    await fireEvent.input(screen.getByLabelText('Password for the test'), {
      target: { value: 'right' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test a login' }));
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());

    const tests = sent.filter((request) => request.path.startsWith('test/'));
    expect(tests.map((request) => [request.path, request.body])).toEqual([
      ['test/ldap', undefined],
      ['test/ldap/login', { username: 'robin', password: 'wrong' }],
      ['test/ldap/login', { username: 'robin', password: 'right' }],
    ]);
    // The typed values are no settings, so nothing waits to be saved.
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('lists the Plex servers of the token with their ids, or says why not', async () => {
    let known = false;
    await showLogin({
      'GET plex/servers': () =>
        known
          ? {
              data: [
                {
                  name: 'Media',
                  address: '203.0.113.5',
                  machineIdentifier: 'made-up-id',
                  owned: 1,
                },
                {
                  name: 'Shared',
                  address: '198.51.100.9',
                  machineIdentifier: 'other-id',
                  owned: 0,
                },
              ],
            }
          : { status: 422, message: 'Plex Token cannot be empty' },
    });
    const button = screen.getByRole('button', { name: 'List the Plex servers' });

    await fireEvent.click(button);
    expect(await screen.findByRole('alert')).toHaveTextContent('Plex Token cannot be empty');

    known = true;
    await fireEvent.click(button);
    const row = (await screen.findByRole('rowheader', { name: 'Media' })).closest('tr');
    expect(row).toHaveTextContent('made-up-id');
    expect(row).toHaveTextContent('This account');
    expect(screen.getByRole('rowheader', { name: 'Shared' }).closest('tr')).toHaveTextContent(
      'Another account',
    );
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
