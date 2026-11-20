import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import Homepage from './Homepage.svelte';
import Overseerr from './Overseerr.svelte';

function request(id: number, title: string, more: Record<string, unknown> = {}) {
  return {
    id: id + 800,
    request_id: id,
    type: 'movie',
    title,
    overview: `The story of ${title}.`,
    poster: 'https://image.tmdb.org/t/p/w300/poster.jpg',
    background: '',
    approved: false,
    available: false,
    denied: false,
    deniedReason: null,
    user: 'pat',
    userAlias: 'Pat the alias',
    request_date: '2020-01-05T10:00:00',
    release_date: '2019-05-01T00:00:00',
    icon: 'mdi mdi-filmstrip',
    color: 'palette-Deep-Purple-900 bg white',
    ...more,
  };
}

const LIST = {
  count: { movie: 2, tv: 1, limit: 50, offset: 0 },
  content: [
    request(32, 'The Show', { type: 'tv', icon: 'mdi mdi-television' }),
    request(33, 'Denied Film', { denied: true, poster: 'plugins/images/homepage/no-list.png' }),
    request(31, 'The Film', { approved: true, available: true }),
  ],
};

function launch(groupID: number) {
  return {
    'GET launch': {
      data: {
        user: { username: 'zoe', group: 'User', groupID, loggedin: true },
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
        tabs: [],
        categories: [],
      },
    },
  };
}

function rowTexts(list: HTMLElement) {
  return within(list)
    .getAllByRole('listitem')
    .map((row) => row.textContent?.replace(/\s+/g, ' ').trim());
}

describe('Ombi', () => {
  beforeEach(async () => {
    stubApi(launch(4));
    await session.load();
  });

  it('shows the requests of Ombi with their states', async () => {
    const sent = stubApi({ 'GET homepage/ombi/requests': { data: LIST } });
    render(Overseerr, { service: 'ombi' });

    const list = await screen.findByRole('list', { name: 'Requests' });
    expect(rowTexts(list)).toEqual([
      'The Show Waiting',
      'D Denied Film Denied',
      'The Film Available',
    ]);
    // The requests of Overseerr are not asked for.
    expect(sent.map((one) => one.path)).toEqual(['homepage/ombi/requests']);
  });

  it('leaves out at the start what the settings switch off', async () => {
    stubApi({ 'GET homepage/ombi/requests': { data: LIST } });
    render(Overseerr, {
      service: 'ombi',
      options: { alias: false, filters: { denied: false, available: false } },
    });

    const list = await screen.findByRole('list', { name: 'Requests' });
    expect(rowTexts(list)).toEqual(['The Show Waiting']);
    // The person can switch a state on again.
    await fireEvent.click(screen.getByLabelText('Denied'));
    expect(rowTexts(list)).toHaveLength(2);
  });

  it('shows the user name, or the alias when the settings say so', async () => {
    stubApi({ 'GET homepage/ombi/requests': { data: LIST } });
    const first = render(Overseerr, { service: 'ombi', options: { alias: false } });
    await fireEvent.click(await screen.findByRole('button', { name: /The Show/ }));
    let dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('Requested by pat');
    expect(dialog.textContent).not.toContain('Pat the alias');
    expect(dialog.textContent).toContain('Series');
    // A user gets no actions.
    expect(within(dialog).queryByRole('button', { name: 'Approve', hidden: true })).toBeNull();
    first.unmount();

    render(Overseerr, { service: 'ombi', options: { alias: true } });
    await fireEvent.click(await screen.findByRole('button', { name: /The Show/ }));
    dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('Requested by Pat the alias');
  });

  it('lets an Admin change a request through the routes of Ombi', async () => {
    stubApi(launch(0));
    await session.load();
    const sent = stubApi({
      'GET homepage/ombi/requests': { data: LIST },
      'PUT homepage/ombi/requests/tv/32/deny': { message: 'Ombi Request has been denied' },
    });
    render(Overseerr, { service: 'ombi' });
    await fireEvent.click(await screen.findByRole('button', { name: /The Show/ }));
    const dialog = await screen.findByRole('dialog', { hidden: true });

    await fireEvent.click(within(dialog).getByRole('button', { name: 'Deny', hidden: true }));

    expect(await screen.findByText('Ombi Request has been denied')).toBeTruthy();
    expect(sent.filter((one) => one.method === 'PUT')).toHaveLength(1);
    // The list is loaded again.
    expect(sent.filter((one) => one.method === 'GET').length).toBeGreaterThanOrEqual(2);
  });

  it('is an item of the homepage, with the options that the server names', async () => {
    stubApi({
      'GET homepage/items': {
        data: [
          {
            key: 'homepageOrderombi',
            kind: 'ombi',
            id: '',
            options: { alias: true, filters: { available: false } },
          },
        ],
      },
      'GET homepage/ombi/requests': { data: LIST },
    });
    render(Homepage, { title: 'Homepage' });

    const list = await screen.findByRole('list', { name: 'Requests' });
    expect(rowTexts(list)).toEqual(['The Show Waiting', 'D Denied Film Denied']);
    await fireEvent.click(screen.getByRole('button', { name: /The Show/ }));
    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('Requested by Pat the alias');
  });

  it('names Ombi when the requests cannot be loaded', async () => {
    stubApi({
      'GET homepage/ombi/requests': { status: 500, message: 'Ombi cannot be reached' },
    });
    render(Overseerr, { service: 'ombi' });

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Ombi: Ombi cannot be reached',
    );
  });
});
