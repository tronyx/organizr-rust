import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import RequestSearch from './RequestSearch.svelte';

const FOUND = {
  page: 1,
  totalPages: 2,
  results: [
    {
      id: 815,
      type: 'movie',
      title: 'The Film',
      year: '2019',
      poster: 'plugins/images/homepage/no-list.png',
      overview: 'A made-up film.',
      status: null,
    },
    {
      id: 816,
      type: 'tv',
      title: 'The Show',
      year: '2020',
      poster: 'plugins/images/homepage/no-list.png',
      overview: '',
      status: 'available',
    },
  ],
};

describe('RequestSearch', () => {
  it('searches the request service and requests a film that it does not have', async () => {
    const sent = stubApi({
      'GET homepage/overseerr/search/The%20F': { data: FOUND },
      'GET homepage/overseerr/search/The%20F/2': { data: { ...FOUND, page: 2, results: [] } },
      'POST homepage/overseerr/requests/movie/815': { message: 'The Film is requested' },
    });
    const onrequested = vi.fn();
    render(RequestSearch, { service: 'overseerr', onrequested });

    await fireEvent.click(screen.getByRole('button', { name: 'Request a Show or Movie' }));
    await fireEvent.input(screen.getByRole('searchbox', { name: 'Request a Show or Movie' }), {
      target: { value: 'The F' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Search' }));

    const results = await screen.findByRole('list', { name: 'Results in Overseerr' });
    expect(within(results).getAllByRole('listitem')).toHaveLength(2);
    expect(results).toHaveTextContent('Available');

    // What the service has already cannot be requested again.
    await fireEvent.click(within(results).getByRole('button', { name: /The Show/ }));
    expect(screen.queryByRole('button', { name: 'Request' })).toBeNull();
    await fireEvent.click(screen.getAllByRole('button', { name: 'Close' }).at(-1) as HTMLElement);

    await fireEvent.click(within(results).getByRole('button', { name: /The Film/ }));
    expect(screen.getByText('A made-up film.')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Request' }));
    await waitFor(() => expect(onrequested).toHaveBeenCalledWith('The Film is requested'));
    expect(sent.some((request) => request.method === 'POST')).toBe(true);
  });

  it('pages through a long list of matches', async () => {
    const sent = stubApi({
      'GET homepage/ombi/search/Film': { data: FOUND },
      'GET homepage/ombi/search/Film/2': { data: { ...FOUND, page: 2, results: [] } },
    });
    render(RequestSearch, { service: 'ombi', onrequested: vi.fn() });

    await fireEvent.click(screen.getByRole('button', { name: 'Request a Show or Movie' }));
    await fireEvent.input(screen.getByRole('searchbox', { name: 'Request a Show or Movie' }), {
      target: { value: 'Film' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Search' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Next' }));

    expect(await screen.findByText('No results for Film')).toBeInTheDocument();
    expect(sent.map((request) => request.path)).toContain('homepage/ombi/search/Film/2');
  });
});
