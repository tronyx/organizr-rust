import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../test-helpers';
import PlexLibraries from './PlexLibraries.svelte';

function library(id: string, title: string, shared: boolean, changeable: boolean) {
  return { id, title, kind: 'movie', shared, changeable };
}

const SHARES = {
  manager: false,
  shares: [
    {
      id: '9001',
      username: 'Robin',
      email: 'robin@example.com',
      libraries: [library('7001', 'Films', true, false), library('7002', 'Shows', false, true)],
    },
  ],
};

describe('PlexLibraries', () => {
  it('shows each library of a share as a switch, and only the changeable ones work', async () => {
    stubApi({ 'GET plugins/plexlibraries/shares': { data: SHARES } });
    render(PlexLibraries, { title: 'Plex Libraries' });

    const share = await screen.findByRole('region', { name: 'Robin' });
    const films = within(share).getByRole('checkbox', { name: 'Films' });
    const shows = within(share).getByRole('checkbox', { name: 'Shows' });
    expect(films).toBeChecked();
    expect(films).toBeDisabled();
    expect(shows).not.toBeChecked();
    expect(shows).toBeEnabled();
  });

  it('sends the libraries of the share after a switch', async () => {
    const sent = stubApi({
      'GET plugins/plexlibraries/shares': { data: SHARES },
      'PUT plugins/plexlibraries/shares/9001': { message: 'The libraries are changed' },
    });
    render(PlexLibraries, { title: 'Plex Libraries' });

    const share = await screen.findByRole('region', { name: 'Robin' });
    await fireEvent.click(within(share).getByRole('checkbox', { name: 'Shows' }));

    const put = sent.find((request) => request.method === 'PUT');
    expect(put?.body).toEqual({ libraries: ['7001', '7002'] });
    expect(await screen.findByRole('status')).toHaveTextContent('The libraries are changed');
  });

  it('says so when the person has no share of the Plex server', async () => {
    stubApi({ 'GET plugins/plexlibraries/shares': { data: { manager: false, shares: [] } } });
    render(PlexLibraries, { title: 'Plex Libraries' });

    expect(await screen.findByText(/no share of the Plex server/)).toBeTruthy();
  });
});
