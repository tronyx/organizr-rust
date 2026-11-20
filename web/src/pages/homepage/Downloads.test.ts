import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import Nzbget from './Nzbget.svelte';
import Qbittorrent from './Qbittorrent.svelte';
import Sabnzbd from './Sabnzbd.svelte';
import Transmission from './Transmission.svelte';

const QUEUE = {
  content: {
    queueItems: {
      queue: {
        paused: false,
        speed: '1.2 M',
        slots: [
          {
            nzo_id: 'nzo_one',
            filename: 'The.Show.S02E03',
            status: 'Downloading',
            percentage: '40',
            timeleft: '0:10:00',
          },
          { nzo_id: 'nzo_two', filename: 'A.Film', status: 'Paused', percentage: '0' },
        ],
      },
    },
    historyItems: {
      history: {
        slots: [{ nzo_id: 'old', name: 'An.Old.Film', status: 'Completed', size: '4 GB' }],
      },
    },
  },
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

describe('Sabnzbd', () => {
  beforeEach(async () => {
    stubApi(launch(4));
    await session.load();
  });

  it('shows the queue and the last downloads', async () => {
    stubApi({ 'GET homepage/sabnzbd/queue': { data: QUEUE } });
    render(Sabnzbd);

    const table = await screen.findByRole('table', { name: 'Downloads of SABnzbd' });
    const [, first] = within(table).getAllByRole('row') as [HTMLElement, HTMLElement];
    expect(first.textContent).toContain('The.Show.S02E03');
    expect(first.textContent).toContain('40 %');
    expect(first.textContent).toContain('0:10:00');
    expect(screen.getByText('1.2 MB/s')).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Last downloads of SABnzbd' }).textContent).toContain(
      'An.Old.Film',
    );
    // A user sees the queue and cannot stop it.
    expect(screen.queryByRole('button', { name: 'Pause all' })).toBeNull();
  });

  it('lets an Admin pause the queue and resume one download', async () => {
    stubApi(launch(0));
    await session.load();
    const sent = stubApi({
      'GET homepage/sabnzbd/queue': { data: QUEUE },
      'POST homepage/sabnzbd/queue/pause': { data: { content: { status: true } } },
      'POST homepage/sabnzbd/queue/resume': { data: { content: { status: true } } },
    });
    render(Sabnzbd);

    await fireEvent.click(await screen.findByRole('button', { name: 'Pause all' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Resume A.Film' }));

    const changes = sent.filter((request) => request.method === 'POST');
    expect(changes.map((request) => [request.path, request.body])).toEqual([
      ['homepage/sabnzbd/queue/pause', { target: 'main' }],
      ['homepage/sabnzbd/queue/resume', { target: 'nzo_two' }],
    ]);
    // The queue is loaded again after each change.
    await waitFor(() => expect(sent.filter((request) => request.method === 'GET')).toHaveLength(3));
  });

  it('says so when SABnzbd gave no answer', async () => {
    stubApi({ 'GET homepage/sabnzbd/queue': { data: { content: false } } });
    render(Sabnzbd);

    expect(await screen.findByText(/SABnzbd gave no answer/)).toBeTruthy();
  });
});

describe('Qbittorrent', () => {
  it('shows each torrent with its state in words and the time that is left', async () => {
    stubApi({
      'GET homepage/qbittorrent/queue': {
        data: {
          content: {
            queueItems: [
              { name: 'The.Show', state: 'downloading', progress: 0.4, eta: 3720 },
              { name: 'An.Old.Film', state: 'stalledUP', progress: 1, eta: 8640000 },
              { name: 'A.New.Kind', state: 'someNewState', progress: 0, eta: 8640000 },
            ],
            historyItems: false,
          },
        },
      },
    });
    render(Qbittorrent);

    const table = await screen.findByRole('table', { name: 'Torrents of qBittorrent' });
    const [, first, second, third] = within(table).getAllByRole('row') as [
      HTMLElement,
      HTMLElement,
      HTMLElement,
      HTMLElement,
    ];
    expect(first.textContent).toContain('Downloading');
    expect(first.textContent).toContain('40 %');
    expect(first.textContent).toContain('1 h 2 min');
    expect(second.textContent).toContain('Uploading');
    expect(second.textContent).toContain('100 %');
    // A state without a word shows as it came.
    expect(third.textContent).toContain('someNewState');
  });

  it('says so when there are no torrents', async () => {
    stubApi({
      'GET homepage/qbittorrent/queue': {
        data: { content: { queueItems: [], historyItems: false } },
      },
    });
    render(Qbittorrent);

    expect(await screen.findByText('There are no torrents to show.')).toBeTruthy();
  });
});

describe('Nzbget', () => {
  it('shows the queue with how far each download is, and the history', async () => {
    stubApi({
      'GET homepage/nzbget/queue': {
        data: {
          content: {
            queueItems: {
              result: [
                {
                  NZBName: 'The.Show',
                  Status: 'DOWNLOADING',
                  FileSizeMB: 1000,
                  RemainingSizeMB: 250,
                },
                { NZBName: 'A.Film', Status: 'QUEUED', FileSizeMB: 0, RemainingSizeMB: 0 },
              ],
            },
            historyItems: { result: [{ Name: 'An.Old.Film', Status: 'SUCCESS/ALL' }] },
          },
        },
      },
    });
    render(Nzbget);

    const queue = await screen.findByRole('table', { name: 'Queue of NZBGet' });
    const rows = within(queue).getAllByRole('row');
    expect(rows[1]?.textContent).toContain('Downloading');
    expect(rows[1]?.textContent).toContain('75 %');
    // A download without a size is at the start.
    expect(rows[2]?.textContent).toContain('0 %');
    const history = screen.getByRole('table', { name: 'History of NZBGet' });
    expect(history.textContent).toContain('An.Old.Film');
    expect(history.textContent).toContain('Success, all');
  });

  it('says so when NZBGet gave no answer, and when the queue is empty', async () => {
    stubApi({ 'GET homepage/nzbget/queue': { data: { content: false } } });
    const { unmount } = render(Nzbget);
    expect(await screen.findByText(/NZBGet gave no answer/)).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/nzbget/queue': { data: { content: { queueItems: { result: [] } } } },
    });
    render(Nzbget);
    expect(await screen.findByText('The queue is empty.')).toBeTruthy();
    expect(screen.queryByRole('table')).toBeNull();
  });
});

describe('Transmission', () => {
  it('shows each torrent with its state in words and the time that is left', async () => {
    stubApi({
      'GET homepage/transmission/queue': {
        data: {
          content: {
            queueItems: [
              { name: 'The.Show', status: 4, percentDone: 0.4, eta: 3720 },
              { name: 'An.Old.Film', status: 6, percentDone: 1, eta: -1 },
              {
                name: 'A.Broken.One',
                status: 0,
                percentDone: 0.1,
                eta: -2,
                errorString: 'No data',
              },
              { name: 'A.New.Kind', status: 9, percentDone: 0, eta: -1 },
            ],
            historyItems: false,
          },
        },
      },
    });
    render(Transmission);

    const table = await screen.findByRole('table', { name: 'Torrents of Transmission' });
    const rows = within(table).getAllByRole('row');
    expect(rows[1]?.textContent).toContain('Downloading');
    expect(rows[1]?.textContent).toContain('40 %');
    expect(rows[1]?.textContent).toContain('1 h 2 min');
    expect(rows[2]?.textContent).toContain('Uploading');
    expect(rows[3]?.textContent).toContain('Error: No data');
    // A state without a word shows as its number.
    expect(rows[4]?.textContent).toContain('9');
  });

  it('says so when there are no torrents, and when Transmission gave no answer', async () => {
    stubApi({
      'GET homepage/transmission/queue': {
        data: { content: { queueItems: [], historyItems: false } },
      },
    });
    const { unmount } = render(Transmission);
    expect(await screen.findByText('There are no torrents to show.')).toBeTruthy();
    unmount();

    stubApi({ 'GET homepage/transmission/queue': { data: { content: false } } });
    render(Transmission);
    expect(await screen.findByText(/Transmission gave no answer/)).toBeTruthy();
  });
});
