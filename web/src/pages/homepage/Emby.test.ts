import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { PlexItem } from '../../lib/plex';
import { linkOf, pictureAddress, streamText } from '../../lib/plex';
import { stubApi } from '../../test-helpers';
import Plex from './Plex.svelte';

/** An item as the server builds it for Emby and for Jellyfin. */
function item(source: string, title: string, more: Partial<PlexItem> = {}): PlexItem {
  return {
    type: 'movie',
    title,
    secondaryTitle: '',
    summary: '',
    uid: '815',
    metadataKey: '815',
    imageURL: `api/v2/homepage/image?source=${source}&type=Primary&img=815&height=600&width=400&key=815-list`,
    nowPlayingImageURL: 'plugins/images/homepage/no-np.png',
    nowPlayingTitle: title,
    nowPlayingBottom: 2019,
    watched: 0,
    state: 'play',
    sessionType: 'Direct Playing',
    user: '',
    address: ` https://media.example.com/item?id=815`,
    ...more,
  };
}

describe.each([
  ['emby', 'Emby'],
  ['jellyfin', 'Jellyfin'],
] as const)('%s on the homepage', (service, name) => {
  it('shows each stream with its state, its user and how it plays', async () => {
    stubApi({
      [`GET homepage/${service}/streams`]: {
        data: {
          content: [
            item(service, 'The Show', {
              nowPlayingTitle: 'The Show - The One',
              nowPlayingBottom: 'S2 · E3',
              watched: 25,
              state: 'pause',
              user: 'robin',
              sessionType: 'Transcoding',
              userStream: { platform: 'Web', videoResolution: 1920 },
            }),
            item(service, 'The Film'),
          ],
        },
      },
    });
    render(Plex, { part: 'streams', service });

    const list = await screen.findByRole('list', { name: `Active ${name} Streams` });
    const [first, second] = within(list).getAllByRole('listitem') as [HTMLElement, HTMLElement];
    expect(first.textContent).toContain('The Show - The One');
    expect(first.textContent).toContain('S2 · E3');
    expect(first.textContent).toContain('Paused');
    expect(first.textContent).toContain('robin');
    expect(first.textContent).toContain('Transcoding · Web');
    expect(within(first).getByRole('progressbar').getAttribute('value')).toBe('25');
    expect(second.textContent).toContain('Playing');
    expect(second.textContent).toContain('2019');
    expect(
      screen.getByRole('heading', { name: new RegExp(`Active ${name} Streams`) }).textContent,
    ).toContain('2');
  });

  it('shows nothing when nothing plays', async () => {
    const sent = stubApi({ [`GET homepage/${service}/streams`]: { data: { content: [] } } });
    const { container } = render(Plex, { part: 'streams', service });

    await waitFor(() => expect(sent.length).toBeGreaterThan(0));
    await Promise.resolve();
    expect(container.textContent.trim()).toBe('');
  });

  it('shows the new items as posters, and the details of one from the server', async () => {
    const sent = stubApi({
      [`GET homepage/${service}/recent`]: { data: { content: [item(service, 'The Film')] } },
      [`POST homepage/${service}/metadata`]: {
        data: {
          content: [
            item(service, 'The Film', {
              metadata: {
                summary: 'A long plot.',
                year: 2019,
                rating: '7.5',
                genres: ['Drama'],
                actors: [{ name: 'An Actor', role: 'The Hero' }],
              },
            }),
          ],
        },
      },
    });
    render(Plex, { part: 'recent', service });

    expect(await screen.findByRole('heading', { name: /Recently Added/ })).toBeTruthy();
    const poster = await screen.findByRole('button', { name: /The Film/ });
    expect(poster.querySelector('img')?.getAttribute('src')).toContain(`source=${service}`);
    await fireEvent.click(poster);

    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(await within(dialog).findByText('A long plot.')).toBeTruthy();
    expect(dialog.textContent).toContain('2019');
    expect(dialog.textContent).toContain('Drama');
    expect(dialog.textContent).toContain('With An Actor');
    expect(sent.find((request) => request.method === 'POST')?.body).toEqual({ key: '815' });
    // The address of the settings of PHP Organizr starts with a space.
    expect(
      within(dialog)
        .getByRole('link', { name: `Open in ${name}`, hidden: true })
        .getAttribute('href'),
    ).toBe('https://media.example.com/item?id=815');
  });

  it('says so when nothing was added, and shows the error of the server', async () => {
    stubApi({ [`GET homepage/${service}/recent`]: { data: { content: [] } } });
    const { unmount } = render(Plex, { part: 'recent', service });
    expect(await screen.findByText('Nothing was added.')).toBeTruthy();
    unmount();

    stubApi({
      [`GET homepage/${service}/streams`]: {
        status: 500,
        result: 'error',
        message: `${name} Error Occurred`,
      },
    });
    render(Plex, { part: 'streams', service });
    expect((await screen.findByRole('alert')).textContent).toContain(`${name}: `);
  });
});

describe('helpers for Emby and Jellyfin', () => {
  it('know a picture without a type, which the server does not have', () => {
    expect(
      pictureAddress('api/v2/homepage/image?source=emby&type=&img=815&height=600&width=400'),
    ).toBeNull();
    expect(pictureAddress('data/cache/jellyfin-815-list.jpg')).toBe(
      'data/cache/jellyfin-815-list.jpg',
    );
  });

  it('leave the width of the picture out of the text of a stream', () => {
    expect(
      streamText(item('emby', 'A', { userStream: { videoResolution: 1920, product: 'Web' } })),
    ).toBe('Direct Playing · Web');
  });

  it('give a link only for a web address', () => {
    expect(linkOf(item('emby', 'A', { address: null }))).toBeNull();
    expect(linkOf(item('emby', 'A', { address: 'javascript:alert(1)' }))).toBeNull();
    expect(linkOf(item('emby', 'A', { address: ' http://emby.example.com/a' }))).toBe(
      'http://emby.example.com/a',
    );
  });
});
