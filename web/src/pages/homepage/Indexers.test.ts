import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { at, stubApi } from '../../test-helpers';
import Indexers from './Indexers.svelte';

const JACKETT = {
  Results: [
    {
      PublishDate: '2026-10-20T08:30:00+00:00',
      Tracker: 'Tracker two',
      Title: 'A Film 2024 720p',
      Details: null,
      Size: 1073741824,
      Files: null,
      Grabs: null,
      Seeders: 3,
      Peers: 1,
      MagnetUri: 'magnet:?xt=urn:btih:00',
      BlackholeLink: null,
      Guid: 'magnet:?xt=urn:btih:00',
      Link: null,
    },
    {
      PublishDate: '2026-11-01T10:00:00+00:00',
      Tracker: 'Tracker one',
      Title: 'A Film 2024 1080p',
      Details: 'https://tracker.example.com/details/1',
      Size: 4294967296,
      Files: 3,
      Grabs: 120,
      Seeders: 45,
      Peers: 7,
      MagnetUri: null,
      BlackholeLink: 'http://127.0.0.1:9117/bh/tracker/?jackett_apikey=&path=abc&file=A+Film',
      Guid: 'https://tracker.example.com/details/1',
      Link: 'http://127.0.0.1:9117/dl/tracker/?jackett_apikey=&path=abc&file=A+Film',
    },
  ],
};

const PROWLARR = [
  {
    guid: 'https://tracker.example.com/details/1',
    indexerId: 5,
    indexer: 'Tracker one',
    title: 'A Film 2024 1080p',
    size: 4294967296,
    publishDate: '2026-11-01T10:00:00Z',
    downloadUrl: 'http://127.0.0.1:9696/5/download?apikey=&link=abc',
    infoUrl: 'https://tracker.example.com/details/1',
    grabs: 120,
    seeders: 45,
    leechers: 7,
  },
  {
    guid: 'https://tracker.example.com/details/2',
    indexerId: 5,
    indexer: 'Tracker one',
    title: 'A Film 2024 2160p',
    size: 1536,
    publishDate: '2026-11-02T10:00:00Z',
    infoUrl: null,
    grabs: 1,
    seeders: 90,
    leechers: 2,
  },
];

async function searchFor(text: string) {
  await fireEvent.input(screen.getByRole('searchbox'), { target: { value: text } });
  await fireEvent.click(screen.getByRole('button', { name: 'Search' }));
}

describe('Indexers', () => {
  it('searches Jackett and shows the newest release first', async () => {
    const sent = stubApi({ 'GET homepage/jackett/A%20Film': { data: { content: JACKETT } } });
    render(Indexers, { service: 'jackett' });
    expect(screen.getByRole('heading', { name: 'Jackett' })).toBeTruthy();
    expect(screen.queryByRole('table')).toBeNull();

    await searchFor(' A Film ');

    const table = await screen.findByRole('table', { name: 'Releases that Jackett found' });
    const rows = within(table).getAllByRole('row');
    expect(rows).toHaveLength(3);
    expect(at(rows, 1).textContent).toContain('A Film 2024 1080p');
    expect(at(rows, 1).textContent).toContain('Tracker one');
    expect(at(rows, 1).textContent).toContain('4.0 GiB');
    expect(at(rows, 1).textContent).toContain('120');
    expect(sent.map((request) => request.path)).toEqual(['homepage/jackett/A%20Film']);

    // The name opens the page of the release. Without the black hole switch, the
    // download is a link that needs no key: here the page, and the magnet link.
    const name = within(at(rows, 1)).getByRole('link', { name: 'A Film 2024 1080p' });
    expect(name.getAttribute('href')).toBe('https://tracker.example.com/details/1');
    expect(
      within(at(rows, 1))
        .getByRole('link', { name: 'Open the link of A Film 2024 1080p' })
        .getAttribute('href'),
    ).toBe('https://tracker.example.com/details/1');
    expect(
      within(at(rows, 2))
        .getByRole('link', { name: 'Open the link of A Film 2024 720p' })
        .getAttribute('href'),
    ).toBe('magnet:?xt=urn:btih:00');
    expect(screen.queryByRole('button', { name: /^Download/ })).toBeNull();
  });

  it('sends a release of Jackett to its black hole folder', async () => {
    const sent = stubApi({
      'GET homepage/jackett/film': { data: { content: JACKETT } },
      'POST homepage/jackett/download/': { data: { content: { result: 'success' } } },
    });
    render(Indexers, { service: 'jackett', blackHole: true });
    await searchFor('film');

    await fireEvent.click(
      await screen.findByRole('button', { name: 'Download A Film 2024 1080p with Jackett' }),
    );

    expect(await screen.findByText('Jackett got the release: A Film 2024 1080p')).toBeTruthy();
    expect(at(sent, 1).body).toEqual({ url: '/bh/tracker/?jackett_apikey=&path=abc&file=A+Film' });
  });

  it('searches Prowlarr, shows the release with the most seeders first, and grabs one', async () => {
    const sent = stubApi({
      'GET homepage/prowlarr/film': { data: { content: PROWLARR } },
      'POST homepage/prowlarr/download/': (request) =>
        (request.body as { guid: string }).guid.endsWith('/2')
          ? { status: 400, message: 'Unknown error' }
          : { data: { content: '{}' } },
    });
    render(Indexers, { service: 'prowlarr', blackHole: true });
    await searchFor('film');

    const table = await screen.findByRole('table', { name: 'Releases that Prowlarr found' });
    const rows = within(table).getAllByRole('row');
    expect(at(rows, 0).textContent).toContain('Indexer');
    expect(at(rows, 0).textContent).not.toContain('Files');
    expect(at(rows, 1).textContent).toContain('A Film 2024 2160p');
    expect(at(rows, 1).textContent).toContain('1.5 KiB');

    await fireEvent.click(
      screen.getByRole('button', { name: 'Download A Film 2024 1080p with Prowlarr' }),
    );
    expect(await screen.findByText(/Prowlarr got the release/)).toBeTruthy();
    expect(at(sent, 1).body).toEqual({
      guid: 'https://tracker.example.com/details/1',
      indexerId: 5,
    });

    await fireEvent.click(
      screen.getByRole('button', { name: 'Download A Film 2024 2160p with Prowlarr' }),
    );
    expect((await screen.findByRole('alert')).textContent).toContain(
      'Error downloading torrent: Unknown error',
    );
  });

  it('shows only a link to the page of a release of Prowlarr without the switch', async () => {
    stubApi({ 'GET homepage/prowlarr/film': { data: { content: PROWLARR } } });
    render(Indexers, { service: 'prowlarr' });
    await searchFor('film');

    const table = await screen.findByRole('table');
    const rows = within(table).getAllByRole('row');
    expect(at(rows, 1).textContent).toContain('No download link');
    expect(
      within(at(rows, 2))
        .getByRole('link', { name: 'Open the link of A Film 2024 1080p' })
        .getAttribute('href'),
    ).toBe('https://tracker.example.com/details/1');
    // The download link of Prowlarr needs its key, and is not shown.
    expect(table.innerHTML).not.toContain('9696');
  });

  it('says so when nothing was found, when the service gave no answer, and clears', async () => {
    stubApi({
      'GET homepage/jackett/none': { data: { content: { Results: [] } } },
      'GET homepage/jackett/broken': { data: { content: false } },
      'GET homepage/jackett/off': {
        status: 409,
        message: 'homepageJackettEnabled module is not enabled',
      },
    });
    render(Indexers, { service: 'jackett' });

    await searchFor('none');
    expect(await screen.findByText('No release was found.')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Clear the search' }));
    expect(screen.queryByText('No release was found.')).toBeNull();
    expect((screen.getByRole('searchbox') as HTMLInputElement).value).toBe('');

    await searchFor('broken');
    expect((await screen.findByRole('alert')).textContent).toContain('Jackett gave no answer');
    await searchFor('off');
    expect(await screen.findByText(/homepageJackettEnabled module is not enabled/)).toBeTruthy();
  });
});
