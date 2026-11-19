import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { PlexItem } from '../../lib/plex';
import { pictureAddress, streamText } from '../../lib/plex';
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

    const list = await screen.findByRole('list', { name: 'Playing now' });
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
    expect(screen.getByRole('heading', { name: /Playing now/ }).textContent).toContain('2');
  });

  it('says so when nothing plays', async () => {
    stubApi({ 'GET homepage/plex/streams': { data: { content: [] } } });
    render(Plex, { part: 'streams' });

    expect(await screen.findByText('Nothing plays at this time.')).toBeTruthy();
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
  it('know the picture that stands for no picture', () => {
    expect(pictureAddress('plugins/images/homepage/no-list.png')).toBeNull();
    expect(pictureAddress('')).toBeNull();
    expect(pictureAddress(undefined)).toBeNull();
    expect(pictureAddress('api/v2/homepage/image?source=plex&key=1-list')).toBe(
      'api/v2/homepage/image?source=plex&key=1-list',
    );
  });

  it('say how a stream plays', () => {
    expect(
      streamText(item('A', { userStream: { videoResolution: '4k', product: 'Plex Web' } })),
    ).toBe('4k · Direct Playing · Plex Web');
    expect(streamText(item('A'))).toBe('Direct Playing');
  });
});
