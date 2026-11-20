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

const LIBRARIES = [
  { section_name: 'Old Films', section_type: 'movie', count: 20, plays: 30 },
  {
    section_name: 'Films',
    section_type: 'movie',
    count: 120,
    parent_count: null,
    child_count: '',
    plays: 300,
  },
  {
    section_name: 'Series',
    section_type: 'show',
    count: 10,
    parent_count: 40,
    child_count: 500,
    plays: 900,
  },
];

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

    const films = await screen.findByRole('region', { name: 'Most Watched Movies' });
    expect(
      within(films)
        .getAllByRole('listitem')
        .map((row) => row.textContent),
    ).toEqual(['1 The Film 12', '2 Another Film 9']);
    // The poster that the server keeps is shown.
    expect(films.querySelector('img')?.getAttribute('src')).toBe(
      'data/cache/tautulli-815-list.jpg',
    );
    expect(screen.getByRole('region', { name: 'Most Popular TV Shows' }).textContent).toContain(
      'The Show 4',
    );
    expect(screen.getByRole('region', { name: 'Most Active Users' }).textContent).toContain(
      'Robin',
    );
    expect(screen.getByRole('region', { name: 'Most Active Platforms' }).textContent).toContain(
      'Chrome',
    );
    // The libraries of a kind share a card, the largest first. Series have three numbers.
    expect(
      within(screen.getByRole('region', { name: 'Movie Libraries' }))
        .getAllByRole('listitem')
        .map((row) => row.textContent),
    ).toEqual(['1 Films 120', '2 Old Films 20']);
    expect(screen.getByRole('region', { name: 'TV Show Libraries' }).textContent).toContain(
      '10 / 40 / 500',
    );
    // A statistic without rows, and one that the item does not know, are left out.
    expect(screen.queryByRole('region', { name: 'Most Watched TV Shows' })).toBeNull();
    expect(screen.queryByText('Unknown')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Tautulli' })).toBeTruthy();
  });

  it('leaves out what the options switch off, and shows the user name when they say so', async () => {
    show({ topMovies: false, libraries: false, title: false, friendlyName: false });

    const users = await screen.findByRole('region', { name: 'Most Active Users' });
    expect(users.textContent).toContain('robin');
    expect(users.textContent).not.toContain('Robin');
    expect(screen.queryByRole('region', { name: 'Most Watched Movies' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Movie Libraries' })).toBeNull();
    expect(screen.queryByRole('heading', { level: 2 })).toBeNull();
  });

  it('says so when Tautulli gave nothing', async () => {
    stubApi({ 'GET homepage/tautulli/data': { data: { options: ALL } } });
    render(Tautulli);

    expect(await screen.findByText(/Tautulli gave no statistics/)).toBeTruthy();
  });
});
