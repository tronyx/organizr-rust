import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import Deluge from './Deluge.svelte';
import Downloaders from './Downloaders.svelte';
import Jdownloader from './Jdownloader.svelte';
import Nzbget from './Nzbget.svelte';
import Qbittorrent from './Qbittorrent.svelte';
import Rtorrent from './Rtorrent.svelte';
import Sabnzbd from './Sabnzbd.svelte';
import Transmission from './Transmission.svelte';
import Utorrent from './Utorrent.svelte';

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

describe('Deluge', () => {
  it('shows each torrent with its state, how far it is, and what the tracker said', async () => {
    stubApi({
      'GET homepage/deluge/queue': {
        data: {
          content: {
            queueItems: [
              {
                name: 'The.Show',
                state: 'Downloading',
                progress: 40.5,
                eta: 3720,
                tracker_status: 'Announce OK',
              },
              { name: 'An.Old.Film', state: 'Seeding', progress: 100, eta: 0, tracker_status: '' },
            ],
            historyItems: false,
          },
        },
      },
    });
    render(Deluge);

    const table = await screen.findByRole('table', { name: 'Torrents of Deluge' });
    const rows = within(table).getAllByRole('row');
    expect(rows[1]?.textContent).toContain('Downloading');
    expect(rows[1]?.textContent).toContain('40 %');
    expect(rows[1]?.textContent).toContain('1 h 2 min');
    expect(rows[1]?.textContent).toContain('Announce OK');
    expect(rows[2]?.textContent).toContain('Seeding');
    expect(rows[2]?.textContent).toContain('100 %');
    // The table can scroll, so a long list stays 300 pixels high.
    expect(table.parentElement?.className).toBe('overflow-x-auto');
  });

  it('says so when there are no torrents, and shows the error of the server', async () => {
    stubApi({
      'GET homepage/deluge/queue': { data: { content: { queueItems: [], historyItems: false } } },
    });
    const { unmount } = render(Deluge);
    expect(await screen.findByText('There are no torrents to show.')).toBeTruthy();
    unmount();

    stubApi({ 'GET homepage/deluge/queue': { status: 500, message: 'Login failed' } });
    render(Deluge);
    expect((await screen.findByRole('alert')).textContent).toContain('Deluge: Login failed');
  });
});

describe('Rtorrent', () => {
  it('shows each torrent with its state, its label, and how far it is', async () => {
    stubApi({
      'GET homepage/rtorrent/queue': {
        data: {
          content: {
            queueItems: [
              // rTorrent sends its numbers as text.
              {
                name: 'The.Show',
                status: 'Downloading',
                size: '1000000',
                downloaded: '400000',
                label: 'tv',
              },
              {
                name: 'An.Old.Film',
                status: 'Seeding',
                size: '1000',
                downloaded: '1000',
                label: '',
              },
              { name: 'An.Empty.One', status: 'Stopped', size: '0', downloaded: '0', label: '' },
            ],
            historyItems: false,
          },
        },
      },
    });
    render(Rtorrent);

    const table = await screen.findByRole('table', { name: 'Torrents of rTorrent' });
    const rows = within(table).getAllByRole('row');
    expect(rows[1]?.textContent).toContain('Downloading');
    expect(rows[1]?.textContent).toContain('tv');
    expect(rows[1]?.textContent).toContain('40 %');
    expect(rows[2]?.textContent).toContain('100 %');
    // A torrent without a size is at the start.
    expect(rows[3]?.textContent).toContain('0 %');
    expect(table.parentElement?.className).toBe('overflow-x-auto');
  });

  it('says so when rTorrent gave no answer, and when there are no torrents', async () => {
    stubApi({ 'GET homepage/rtorrent/queue': { data: { content: false } } });
    const { unmount } = render(Rtorrent);
    expect(await screen.findByText(/rTorrent gave no answer/)).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/rtorrent/queue': { data: { content: { queueItems: [], historyItems: false } } },
    });
    render(Rtorrent);
    expect(await screen.findByText('There are no torrents to show.')).toBeTruthy();
  });
});

describe('Utorrent', () => {
  it('shows each torrent with its state, its label, and how far it is', async () => {
    stubApi({
      'GET homepage/utorrent/queue': {
        data: {
          content: {
            queueItems: [
              { Name: 'The.Show', Status: 'Downloading', Percent: '45.3%', Labels: 'tv' },
              { Name: 'An.Old.Film', Status: 'Seeding', Percent: '100.0%', Labels: '' },
              // A message that the server could not read gives no state and no number.
              { Name: 'An.Odd.One', Status: '', Percent: '', Labels: '' },
            ],
          },
        },
      },
    });
    render(Utorrent);

    const table = await screen.findByRole('table', { name: 'Torrents of uTorrent' });
    const rows = within(table).getAllByRole('row');
    expect(rows[1]?.textContent).toContain('Downloading');
    expect(rows[1]?.textContent).toContain('tv');
    expect(rows[1]?.textContent).toContain('45 %');
    expect(rows[2]?.textContent).toContain('100 %');
    expect(rows[3]?.textContent).toContain('0 %');
    expect(table.parentElement?.className).toBe('overflow-x-auto');
  });

  it('says so when uTorrent gave no answer, and when there are no torrents', async () => {
    stubApi({ 'GET homepage/utorrent/queue': { data: { content: false } } });
    const { unmount } = render(Utorrent);
    expect(await screen.findByText(/uTorrent gave no answer/)).toBeTruthy();
    unmount();

    stubApi({ 'GET homepage/utorrent/queue': { data: { content: { queueItems: [] } } } });
    render(Utorrent);
    expect(await screen.findByText('There are no torrents to show.')).toBeTruthy();
  });
});

const PACKAGES = {
  content: {
    queueItems: [
      {
        name: 'A.Package',
        speed: '1.2 MB/s',
        size: '4.00 GB',
        eta: '40m:00s',
        percentage: '25',
        enabled: true,
      },
      {
        name: 'A.Done.Package',
        speed: null,
        size: '1.00 GB',
        eta: null,
        percentage: '100',
        enabled: true,
      },
      {
        name: 'A.Waiting.Package',
        speed: null,
        size: '2.00 GB',
        eta: null,
        percentage: '0',
        enabled: true,
      },
      { name: 'A.Package.That.Is.Off', speed: null, size: '2.00 GB', eta: null, percentage: '0' },
    ],
    grabberItems: [{ name: 'A.New.Package' }],
    encryptedItems: [{ name: 'A.Locked.Package' }],
    offlineItems: [{ name: 'A.Lost.Package' }],
    $status: ['RUNNING', false, false],
  },
};

describe('Jdownloader', () => {
  it('shows the packages that download, and the packages of the link grabber', async () => {
    stubApi({ 'GET homepage/jdownloader/queue': { data: PACKAGES } });
    render(Jdownloader);

    const table = await screen.findByRole('table', { name: 'Packages of JDownloader' });
    const rows = within(table).getAllByRole('row');
    expect(rows).toHaveLength(8);
    expect(rows[1]?.textContent).toContain('1.2 MB/s');
    expect(rows[1]?.textContent).toContain('4.00 GB');
    expect(rows[1]?.textContent).toContain('40m:00s');
    expect(rows[1]?.textContent).toContain('25 %');
    expect(rows[2]?.textContent).toContain('Completed');
    expect(rows[3]?.textContent).toContain('Stopped');
    expect(rows[4]?.textContent).toContain('Disabled');
    expect(rows[5]?.textContent).toContain('Online');
    expect(rows[6]?.textContent).toContain('Encrypted');
    expect(rows[7]?.textContent).toContain('Offline');
    expect(screen.getByText('Downloads: Running')).toBeTruthy();
    expect(table.parentElement?.className).toBe('overflow-x-auto');
  });

  it('says so when the service gave no answer, and when there are no packages', async () => {
    stubApi({ 'GET homepage/jdownloader/queue': { data: { content: false } } });
    const { unmount } = render(Jdownloader);
    expect(await screen.findByText(/JDownloader gave no answer/)).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/jdownloader/queue': {
        data: {
          content: {
            queueItems: [],
            grabberItems: [],
            encryptedItems: [],
            offlineItems: [],
            $status: [null, null, null],
          },
        },
      },
    });
    render(Jdownloader);
    expect(await screen.findByText('There are no packages to show.')).toBeTruthy();
  });
});

describe('Downloaders with the other download clients', () => {
  it('shows a tab with the number of entries for each of them', async () => {
    stubApi({
      'GET homepage/deluge/queue': {
        data: {
          content: { queueItems: [{ name: 'The.Show', progress: 40 }], historyItems: false },
        },
      },
      'GET homepage/rtorrent/queue': { data: { content: false } },
      'GET homepage/utorrent/queue': {
        data: { content: { queueItems: [{ Name: 'One' }, { Name: 'Two' }] } },
      },
      'GET homepage/jdownloader/queue': { data: PACKAGES },
    });
    render(Downloaders, { members: ['deluge', 'rtorrent', 'utorrent', 'jdownloader'] });

    expect(screen.getAllByRole('tab')).toHaveLength(4);
    expect(await screen.findByRole('tab', { name: 'Deluge: 1' })).toBeTruthy();
    expect(screen.getByRole('tab', { name: 'rTorrent: 0' })).toBeTruthy();
    expect(await screen.findByRole('tab', { name: 'uTorrent: 2' })).toBeTruthy();
    const packages = await screen.findByRole('tab', { name: 'JDownloader: 7' });
    expect(document.getElementById('downloader-deluge')?.hidden).toBe(false);
    expect(document.getElementById('downloader-jdownloader')?.hidden).toBe(true);

    await fireEvent.click(packages);
    expect(document.getElementById('downloader-deluge')?.hidden).toBe(true);
    expect(document.getElementById('downloader-jdownloader')?.hidden).toBe(false);
  });
});

describe('Downloaders', () => {
  it('shows a tab with the number of entries for each downloader, and one box at a time', async () => {
    stubApi({
      'GET homepage/sabnzbd/queue': { data: QUEUE },
      'GET homepage/qbittorrent/queue': {
        data: {
          content: {
            queueItems: [
              { name: 'The.Show', state: 'downloading', progress: 0.4, eta: 3720 },
              { name: 'An.Old.Film', state: 'stalledUP', progress: 1, eta: 8640000 },
              { name: 'A.New.Kind', state: 'pausedDL', progress: 0, eta: 8640000 },
            ],
            historyItems: false,
          },
        },
      },
    });
    render(Downloaders, { members: ['sabnzbd', 'qbittorrent', 'something new'] });

    // A member that this interface does not know gets no tab.
    expect(screen.getAllByRole('tab')).toHaveLength(2);
    const torrents = await screen.findByRole('tab', { name: 'qBittorrent: 3' });
    expect(torrents.getAttribute('aria-selected')).toBe('false');
    expect(document.getElementById('downloader-qbittorrent')?.hidden).toBe(true);
    expect(document.getElementById('downloader-sabnzbd')?.hidden).toBe(false);

    await fireEvent.click(torrents);
    expect(torrents.getAttribute('aria-selected')).toBe('true');
    expect(document.getElementById('downloader-qbittorrent')?.hidden).toBe(false);
    expect(document.getElementById('downloader-sabnzbd')?.hidden).toBe(true);
  });
});
