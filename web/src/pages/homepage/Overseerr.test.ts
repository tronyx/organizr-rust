import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
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
    user: 'pat',
    userAlias: 'Pat',
    request_date: '2020-01-05T10:00:00.000Z',
    release_date: '2019-05-01',
    ...more,
  };
}

const LIST = {
  count: { movie: 3, tv: 0 },
  content: [
    request(31, 'The Film', { approved: true, available: true }),
    request(32, 'Waiting Film'),
    request(33, 'Denied Film', { denied: true, poster: 'plugins/images/homepage/no-list.png' }),
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

describe('Overseerr', () => {
  beforeEach(async () => {
    stubApi(launch(4));
    await session.load();
  });

  it('shows each request with its state, and leaves out the states that are switched off', async () => {
    stubApi({ 'GET homepage/overseerr/requests': { data: LIST } });
    render(Overseerr);

    const list = await screen.findByRole('list', { name: 'Requests' });
    const rows = within(list).getAllByRole('listitem');
    expect(rows.map((row) => row.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'The Film Available',
      'Waiting Film Waiting',
      'D Denied Film Denied',
    ]);
    // The picture that stands for "no picture" is not shown.
    expect(rows[2]?.querySelector('img')).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'Filter Requests' }));
    await fireEvent.click(screen.getByLabelText('Denied'));
    expect(within(list).getAllByRole('listitem')).toHaveLength(2);
    // No state of "Approved" is in the list, so it has no switch.
    expect(screen.queryByLabelText('Approved')).toBeNull();
  });

  it('shows the details of a request to a user, without the actions', async () => {
    stubApi({ 'GET homepage/overseerr/requests': { data: LIST } });
    render(Overseerr);
    await fireEvent.click(await screen.findByRole('button', { name: /Waiting Film/ }));

    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('The story of Waiting Film.');
    expect(dialog.textContent).toContain('Requested by Pat');
    expect(dialog.textContent).toContain('2019');
    expect(within(dialog).queryByRole('button', { name: 'Approve', hidden: true })).toBeNull();
    expect(within(dialog).queryByRole('button', { name: 'Delete', hidden: true })).toBeNull();
  });

  it('lets an Admin approve a request, and says what Overseerr answered', async () => {
    stubApi(launch(0));
    await session.load();
    const sent = stubApi({
      'GET homepage/overseerr/requests': { data: LIST },
      'POST homepage/overseerr/requests/movie/32/approve': {
        message: 'Overseerr Request has been approved',
      },
    });
    render(Overseerr);
    await fireEvent.click(await screen.findByRole('button', { name: /Waiting Film/ }));
    const dialog = await screen.findByRole('dialog', { hidden: true });

    await fireEvent.click(within(dialog).getByRole('button', { name: 'Approve', hidden: true }));

    expect(await screen.findByText('Overseerr Request has been approved')).toBeTruthy();
    expect(sent.filter((one) => one.method === 'POST')).toHaveLength(1);
    // The list is loaded again.
    expect(sent.filter((one) => one.method === 'GET').length).toBeGreaterThanOrEqual(2);
  });

  it('offers an Admin the actions that fit the state, and shows why one failed', async () => {
    stubApi(launch(1));
    await session.load();
    const sent = stubApi({
      'GET homepage/overseerr/requests': { data: LIST },
      'POST homepage/overseerr/requests/movie/31/unavailable': {
        status: 500,
        message: 'Request not found.',
      },
    });
    render(Overseerr);
    await fireEvent.click(await screen.findByRole('button', { name: /The Film/ }));
    const dialog = await screen.findByRole('dialog', { hidden: true });
    const button = (name: string) => within(dialog).queryByRole('button', { name, hidden: true });

    // What is available is not approved or denied again.
    expect(button('Approve')).toBeNull();
    expect(button('Deny')).toBeNull();
    expect(button('Delete')).not.toBeNull();
    await fireEvent.click(button('Mark as not available') as HTMLElement);

    expect((await within(dialog).findByRole('alert', { hidden: true })).textContent).toContain(
      'Request not found.',
    );
    expect(sent.some((one) => one.path.endsWith('/31/unavailable'))).toBe(true);
  });

  it('says so when there are no requests', async () => {
    stubApi({
      'GET homepage/overseerr/requests': { data: { count: { movie: 0, tv: 0 }, content: [] } },
    });
    render(Overseerr);

    expect(await screen.findByText('There are no requests to show.')).toBeTruthy();
  });
});
