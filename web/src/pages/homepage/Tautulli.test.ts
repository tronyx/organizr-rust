import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Tautulli from './Tautulli.svelte';

const ALL = {
  libraries: true,
  topMovies: true,
  topTV: true,
  topUsers: true,
  topPlatforms: true,
  popularMovies: true,
  popularTV: true,
  title: true,
  friendlyName: true,
};

const STATISTICS = [
  {
    stat_id: 'top_movies',
    rows: [
      { title: 'The Film', total_plays: 12, thumb: 'data/cache/tautulli-815-list.jpg' },
      { title: 'Another Film', total_plays: 9, thumb: '/library/metadata/816/thumb/1' },
    ],
  },
  { stat_id: 'popular_tv', rows: [{ title: 'The Show', users_watched: 4 }] },
  { stat_id: 'top_users', rows: [{ user: 'robin', friendly_name: 'Robin', total_plays: 30 }] },
  { stat_id: 'top_platforms', rows: [{ platform: 'Chrome', total_plays: 20 }] },
  { stat_id: 'top_tv', rows: [] },
  { stat_id: 'something_new', rows: [{ title: 'Unknown' }] },
];

const LIBRARIES = [{ section_name: 'Films', count: 120, plays: 300 }];

function show(options: Partial<typeof ALL> = {}) {
  stubApi({
    'GET homepage/tautulli/data': {
      data: {
        options: { ...ALL, ...options },
        homestats: { data: STATISTICS },
        libstats: { data: LIBRARIES },
      },
    },
  });
  render(Tautulli);
}

describe('Tautulli', () => {
  it('shows each statistic with its rows, and the libraries', async () => {
    show();

    const films = await screen.findByRole('region', { name: 'Most watched films' });
    expect(
      within(films)
        .getAllByRole('listitem')
        .map((row) => row.textContent),
    ).toEqual(['The Film 12 plays', 'Another Film 9 plays']);
    // The poster that the server keeps is shown.
    expect(films.querySelector('img')?.getAttribute('src')).toBe(
      'data/cache/tautulli-815-list.jpg',
    );
    expect(screen.getByRole('region', { name: 'Most popular series' }).textContent).toContain(
      'The Show 4 users',
    );
    expect(screen.getByRole('region', { name: 'Most active users' }).textContent).toContain(
      'Robin',
    );
    expect(screen.getByRole('region', { name: 'Most used players' }).textContent).toContain(
      'Chrome',
    );
    expect(screen.getByRole('region', { name: 'Libraries' }).textContent).toContain('Films');
    // A statistic without rows, and one that the item does not know, are left out.
    expect(screen.queryByRole('region', { name: 'Most watched series' })).toBeNull();
    expect(screen.queryByText('Unknown')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Tautulli' })).toBeTruthy();
  });

  it('leaves out what the options switch off, and shows the user name when they say so', async () => {
    show({ topMovies: false, libraries: false, title: false, friendlyName: false });

    const users = await screen.findByRole('region', { name: 'Most active users' });
    expect(users.textContent).toContain('robin');
    expect(users.textContent).not.toContain('Robin');
    expect(screen.queryByRole('region', { name: 'Most watched films' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Libraries' })).toBeNull();
    expect(screen.queryByRole('heading', { level: 2 })).toBeNull();
  });

  it('says so when Tautulli gave nothing', async () => {
    stubApi({ 'GET homepage/tautulli/data': { data: { options: ALL } } });
    render(Tautulli);

    expect(await screen.findByText(/Tautulli gave no statistics/)).toBeTruthy();
  });
});
