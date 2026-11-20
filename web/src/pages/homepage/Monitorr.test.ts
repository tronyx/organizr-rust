import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { at, stubApi } from '../../test-helpers';
import Monitorr from './Monitorr.svelte';

const SERVICES = {
  'Backup & Sync': { status: false, sort: 2, image: 'plugins/images/homepage/no-list.png' },
  Plex: {
    status: true,
    sort: 0,
    image: 'data:image/png;base64,/9j/4AAQSkZJRg==',
    link: 'https://plex.example.com/web',
  },
  Router: { status: 'unresponsive', sort: 1, image: 'data:image/png;base64,AAAA', link: '#router' },
};

function show(compact: boolean, titleToggle = true, services: unknown = SERVICES) {
  stubApi({
    'GET homepage/monitorr/data': {
      data: { services, options: { title: 'Monitorr', titleToggle, compact } },
    },
  });
  return render(Monitorr);
}

describe('Monitorr', () => {
  it('shows a tile for each service with its state', async () => {
    const { container } = show(false);

    const list = await screen.findByRole('list', { name: 'Monitorr' });
    const tiles = within(list).getAllByRole('listitem');
    expect(tiles.map((tile) => tile.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'Backup & Sync Offline',
      'Plex Online',
      'Router Unresponsive',
    ]);
    // The title counts the services.
    expect(screen.getByRole('heading').textContent?.replace(/\s+/g, ' ')).toContain('Monitorr: 3');
    // Only an address of the web is a link, and only a picture of the answer shows.
    expect(screen.getByRole('link', { name: 'Plex' }).getAttribute('href')).toBe(
      'https://plex.example.com/web',
    );
    expect(screen.getAllByRole('link')).toHaveLength(1);
    expect(container.querySelectorAll('img')).toHaveLength(2);
  });

  it('shows the small form with a coloured edge for the state', async () => {
    show(true, false);

    const list = await screen.findByRole('list', { name: 'Monitorr' });
    const cards = within(list).getAllByRole('listitem');
    expect(at(cards, 0).style.getPropertyValue('--org-state')).toBe('var(--color-error)');
    expect(at(cards, 1).style.getPropertyValue('--org-state')).toBe('var(--color-success)');
    expect(at(cards, 2).style.getPropertyValue('--org-state')).toBe('var(--color-warning)');
    expect(at(cards, 2).textContent).toContain('Unresponsive');
    expect(screen.queryByRole('heading')).toBeNull();
  });

  it('says so when Monitorr named no service', async () => {
    show(false, true, {});

    expect(await screen.findByText(/Monitorr named no service/)).toBeTruthy();
  });
});
