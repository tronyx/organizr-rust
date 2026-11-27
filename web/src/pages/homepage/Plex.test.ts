import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { PlexItem } from '../../lib/plex';
import {
  NO_POSTER,
  pictureAddress,
  posterAddress,
  streamsSummary,
  streamText,
} from '../../lib/plex';
import { stubApi } from '../../test-helpers';
import Plex from './Plex.svelte';

function item(title: string, more: Partial<PlexItem> = {}): PlexItem {
  return {
    type: 'movie',
    title,
    secondaryTitle: '2019',
    summary: `The story of ${title}.`,
    uid: '815',
    metadataKey: '815',
    imageURL: 'data/cache/plex-815-list.jpg',
    nowPlayingImageURL: 'plugins/images/homepage/no-np.png',
    watched: 0,
    state: 'play',
    sessionType: 'Direct Playing',
    user: '',
    address: 'https://app.plex.tv/web/app#!/server/abc/details?key=/library/metadata/815',
    ...more,
  };
}

describe('Plex', () => {
  it('shows each stream with its state, its user and how it plays', async () => {
    stubApi({
      'GET homepage/plex/streams': {
        data: {
          content: [
            item('The Show', {
              nowPlayingTitle: 'The Show - The One',
              nowPlayingBottom: 'S2 · E3',
              watched: 25,
              state: 'pause',
              user: 'robin',
              sessionType: 'Transcoding',
              userStream: { platform: 'Chrome', videoResolution: '1080' },
            }),
            item('The Film'),
          ],
        },
      },
    });
    render(Plex, { part: 'streams' });

    const list = await screen.findByRole('list', { name: 'Active Plex Streams' });
    const [first, second] = within(list).getAllByRole('listitem') as [HTMLElement, HTMLElement];
    expect(first.textContent).toContain('The Show - The One');
    expect(first.textContent).toContain('S2 · E3');
    expect(first.textContent).toContain('Paused');
    expect(first.textContent).toContain('robin');
    expect(first.textContent).toContain('1080p · Transcoding · Chrome');
    expect(within(first).getByRole('progressbar').getAttribute('value')).toBe('25');
    // The picture that stands for "no picture" is not shown.
    expect(first.querySelector('img')).toBeNull();
    expect(second.textContent).toContain('Playing');
    expect(screen.getByRole('heading', { name: /Active Plex Streams/ }).textContent).toContain('2');
  });

  it('shows nothing when nothing plays', async () => {
    const sent = stubApi({ 'GET homepage/plex/streams': { data: { content: [] } } });
    const { container } = render(Plex, { part: 'streams' });

    await waitFor(() => expect(sent.length).toBeGreaterThan(0));
    await Promise.resolve();
    expect(container.textContent.trim()).toBe('');
  });

  it('shows the new items as posters, and the details of one from Plex', async () => {
    const sent = stubApi({
      'GET homepage/plex/recent': { data: { content: [item('The Film')] } },
      'POST homepage/plex/metadata': {
        data: {
          content: [
            item('The Film', {
              metadata: {
                summary: 'A long plot.',
                year: '2019',
                genres: ['Drama'],
                actors: [{ name: 'An Actor', role: 'The Hero' }],
              },
            }),
          ],
        },
      },
    });
    render(Plex, { part: 'recent' });

    const poster = await screen.findByRole('button', { name: /The Film/ });
    expect(poster.querySelector('img')?.getAttribute('src')).toBe('data/cache/plex-815-list.jpg');
    await fireEvent.click(poster);

    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(await within(dialog).findByText('A long plot.')).toBeTruthy();
    expect(dialog.textContent).toContain('Drama');
    expect(dialog.textContent).toContain('With An Actor');
    expect(sent.find((request) => request.method === 'POST')?.body).toEqual({ key: '815' });
    expect(
      within(dialog).getByRole('link', { name: 'Open in Plex', hidden: true }).getAttribute('href'),
    ).toContain('app.plex.tv');
  });

  it('shows the picture "Poster coming soon" of PHP for a poster that is missing or fails', async () => {
    stubApi({
      'GET homepage/plex/recent': {
        data: {
          content: [
            item('No Picture', { imageURL: 'plugins/images/homepage/no-list.png' }),
            item('Empty Picture', { imageURL: '' }),
            item('Broken Picture'),
          ],
        },
      },
    });
    render(Plex, { part: 'recent' });

    const source = (name: string) =>
      screen.getByRole('button', { name: new RegExp(name) }).querySelector('img');
    await screen.findByRole('button', { name: /No Picture/ });
    expect(source('No Picture')?.getAttribute('src')).toBe(NO_POSTER);
    expect(source('Empty Picture')?.getAttribute('src')).toBe(NO_POSTER);
    // No letter stands in for a poster any more.
    expect(document.querySelector('.org-poster-empty')).toBeNull();
    // A poster that does not load gives way to the same picture.
    const broken = source('Broken Picture') as HTMLImageElement;
    expect(broken.getAttribute('src')).toBe('data/cache/plex-815-list.jpg');
    await fireEvent.error(broken);
    expect(broken.getAttribute('src')).toBe(NO_POSTER);
  });

  it('shows the videos of the playlist that is chosen', async () => {
    stubApi({
      'GET homepage/plex/playlists': {
        data: {
          content: {
            Filmnight: { title: 'Film night!', 0: item('The Film'), 1: item('Another Film') },
            Kids: { title: 'Kids', 0: item('A Film For Children') },
          },
        },
      },
    });
    render(Plex, { part: 'playlists' });

    const first = await screen.findByRole('list', { name: 'Film night!' });
    expect(within(first).getAllByRole('listitem')).toHaveLength(2);

    await fireEvent.change(screen.getByLabelText('Playlist'), { target: { value: 'Kids' } });
    const second = await screen.findByRole('list', { name: 'Kids' });
    expect(second.textContent).toContain('A Film For Children');
  });
});

describe('plex helpers', () => {
  it('shows how a stream plays and its details from the buttons over its picture', async () => {
    stubApi({
      'GET homepage/plex/streams': {
        data: {
          content: [
            item('The Show', {
              sessionType: 'Transcoding',
              bandwidth: 5100,
              bandwidthType: 'wan',
              userAddress: '203.0.113.7',
              nowPlayingOriginalImage: 'api/v2/homepage/image?source=plex&key=815-np$X',
              userStream: {
                stream: 'transcode',
                videoDecision: 'Transcode',
                sourceVideoCodec: 'hevc',
                videoCodec: 'h264',
                videoResolution: '1080',
                audioDecision: 'Direct Play',
                sourceAudioCodec: 'aac',
                audioCodec: 'aac',
                product: 'Plex Web',
                device: 'Firefox',
              },
            }),
          ],
        },
      },
      'POST homepage/plex/metadata': {
        data: { content: [item('The Show', { metadata: { tagline: 'Made-up tagline' } })] },
      },
    });
    render(Plex, { part: 'streams' });

    await fireEvent.click(await screen.findByRole('button', { name: 'How The Show plays' }));
    const info = screen.getByRole('dialog');
    expect(info).toHaveTextContent('5100 kbps');
    expect(info).toHaveTextContent('203.0.113.7');
    expect(info).toHaveTextContent('Transcode (hevc → h264 1080)');
    expect(info).toHaveTextContent('Firefox');
    await fireEvent.click(within(info).getByRole('button', { name: 'Close' }));

    await fireEvent.click(screen.getByRole('button', { name: 'Details of The Show' }));
    expect(await screen.findByText('Made-up tagline')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Open The Show in Plex' })).toHaveAttribute(
      'href',
      expect.stringContaining('/library/metadata/815'),
    );

    await fireEvent.click(screen.getByRole('button', { name: 'Renew the picture of The Show' }));
    const list = screen.getByRole('list', { name: 'Active Plex Streams' });
    expect(list.querySelector('img')?.getAttribute('src')).toContain('815-np$X');
  });

  it('know the picture that stands for no picture', () => {
    expect(pictureAddress('plugins/images/homepage/no-list.png')).toBeNull();
    expect(pictureAddress('')).toBeNull();
    expect(pictureAddress(undefined)).toBeNull();
    expect(pictureAddress('api/v2/homepage/image?source=plex&key=1-list')).toBe(
      'api/v2/homepage/image?source=plex&key=1-list',
    );
    // A poster always has a picture: its own, or the one for a missing poster.
    expect(posterAddress('plugins/images/homepage/no-list.png')).toBe(NO_POSTER);
    expect(posterAddress('')).toBe(NO_POSTER);
    expect(posterAddress(undefined)).toBe(NO_POSTER);
    expect(posterAddress('api/v2/homepage/image?source=emby&type=&img=1')).toBe(NO_POSTER);
    expect(posterAddress('data/cache/plex-1-list.jpg')).toBe('data/cache/plex-1-list.jpg');
  });

  it('say how a stream plays', () => {
    expect(
      streamText(item('A', { userStream: { videoResolution: '4k', product: 'Plex Web' } })),
    ).toBe('4k · Direct Playing · Plex Web');
    expect(streamText(item('A'))).toBe('Direct Playing');
  });

  it('sum up the streams as the info badge of PHP Organizr', () => {
    const streams = [
      item('A', { bandwidth: '8000', bandwidthType: 'wan' }),
      item('B', { bandwidth: 4400, bandwidthType: 'lan', sessionType: 'Transcoding' }),
    ];
    expect(streamsSummary(streams, 'plex')).toBe(
      'Streams: 1 Direct Play(s) & 1 Transcode(s) | Bandwidth: 12.4 Mbps | WAN: 8.0 Mbps | LAN: 4.4 Mbps',
    );
    expect(streamsSummary(streams.slice(0, 1), 'emby')).toBe(
      'Streams: 1 Direct Play(s) | Bandwidth: 8.0 Mbps',
    );
  });

  it('shows an album as a square cover between darkened copies, as PHP Organizr does', async () => {
    stubApi({
      'GET homepage/plex/recent': {
        data: {
          content: [
            item('The Album', { type: 'music', imageURL: 'data/cache/plex-9-list.jpg' }),
            item('The Film'),
          ],
        },
      },
    });
    render(Plex, { part: 'recent' });

    const album = await screen.findByRole('button', { name: /The Album/ });
    expect(album.closest('li')?.classList.contains('org-poster-music')).toBe(true);
    const shades = album.querySelectorAll<HTMLImageElement>('.org-poster-shade');
    expect(shades).toHaveLength(2);
    for (const shade of shades) {
      expect(shade.getAttribute('src')).toBe('data/cache/plex-9-list.jpg');
      expect(shade.getAttribute('alt')).toBe('');
    }
    expect(album.querySelector('.org-poster-picture')?.getAttribute('src')).toBe(
      'data/cache/plex-9-list.jpg',
    );
    // A copy that does not load is left out, not replaced by "Poster coming soon".
    const [top] = shades;
    expect(top).toBeDefined();
    await fireEvent.error(top as HTMLImageElement);
    expect(top?.hidden).toBe(true);

    // A film keeps its one tall poster.
    const film = screen.getByRole('button', { name: /The Film/ });
    expect(film.closest('li')?.classList.contains('org-poster-music')).toBe(false);
    expect(film.querySelectorAll('img')).toHaveLength(1);
  });

  it('filters the recently added items by kind, as PHP Organizr does', async () => {
    stubApi({
      'GET homepage/plex/recent': {
        data: { content: [item('The Film'), item('The Show', { type: 'tv' })] },
      },
    });
    render(Plex, { part: 'recent' });

    const list = await screen.findByRole('list', { name: 'Recently Added' });
    expect(within(list).getAllByRole('listitem')).toHaveLength(2);
    await fireEvent.click(screen.getByRole('button', { name: 'Filter Recently Added' }));
    // Only the kinds that the list has are offered.
    expect(screen.queryByRole('button', { name: 'Music' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Shows' }));
    expect(within(list).getAllByRole('listitem')).toHaveLength(1);
    expect(list.textContent).toContain('The Show');
    await fireEvent.click(screen.getByRole('button', { name: 'All' }));
    expect(within(list).getAllByRole('listitem')).toHaveLength(2);
  });
});
