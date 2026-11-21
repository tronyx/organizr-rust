import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import MediaSearch from './MediaSearch.svelte';

describe('MediaSearch', () => {
  it('searches the media server as the person types, and opens the details of a match', async () => {
    const sent = stubApi({
      'GET homepage/plex/search/The%20Film': {
        data: {
          content: [
            {
              type: 'movie',
              title: 'The Film',
              secondaryTitle: '2019',
              summary: 'A made-up film.',
              uid: '815',
              metadataKey: '815',
              imageURL: 'data/cache/plex-815-list.jpg',
              nowPlayingImageURL: '',
              watched: 0,
              state: '',
              sessionType: '',
              user: '',
              address: null,
            },
          ],
        },
      },
      'POST homepage/plex/metadata': { data: { content: [] } },
    });
    render(MediaSearch, { service: 'plex' });

    await fireEvent.click(screen.getByRole('button', { name: 'Search My Media' }));
    await fireEvent.input(screen.getByRole('searchbox', { name: 'Search My Media' }), {
      target: { value: 'The Film' },
    });

    const results = await screen.findByRole('list', { name: 'Results in Plex' }, { timeout: 2000 });
    expect(sent.filter((request) => request.path.includes('/search/'))).toHaveLength(1);
    await fireEvent.click(within(results).getByRole('button', { name: /The Film/ }));
    expect(await screen.findByText('A made-up film.')).toBeInTheDocument();
  });
});
