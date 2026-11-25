import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { donateOptions, jellyStatOptions, liveTvOptions } from '../../lib/statistics';
import { stubApi } from '../../test-helpers';
import Donate from './Donate.svelte';
import EmbyLiveTv from './EmbyLiveTv.svelte';
import Homepage from './Homepage.svelte';
import JellyStat from './JellyStat.svelte';
import PromPage from './PromPage.svelte';

const text = (element: HTMLElement) => element.textContent?.replace(/\s+/g, ' ') ?? '';

describe('PromPage', () => {
  const options = { title: 'Our services', titleToggle: true, compact: true, showUptime: true };
  const services = [
    { name: 'Blog', status: true, uptime: 99.954 },
    { name: 'Mail', status: false, uptime: null },
  ];

  it('shows each service with its state and its uptime', async () => {
    stubApi({ 'GET homepage/prompage/data': { data: { data: services, options } } });
    render(PromPage);

    const list = await screen.findByRole('list', { name: 'Our services' });
    const [blog, mail] = within(list).getAllByRole('listitem') as [HTMLElement, HTMLElement];
    expect(text(blog)).toContain('Blog');
    expect(text(blog)).toContain('99.95%');
    expect(text(blog)).toContain('Up');
    expect(text(mail)).toContain('Down');
    expect(text(mail)).not.toContain('%');
    expect(screen.getByRole('heading', { name: 'Our services' })).toBeTruthy();
  });

  it('shows the words of the large form, and no uptime and no title when they are off', async () => {
    stubApi({
      'GET homepage/prompage/data': {
        data: {
          data: services,
          options: { ...options, titleToggle: false, compact: false, showUptime: false },
        },
      },
    });
    render(PromPage);

    const list = await screen.findByRole('list');
    expect(text(list)).toContain('UP');
    expect(text(list)).toContain('DOWN');
    expect(text(list)).not.toContain('99.95%');
    expect(screen.queryByRole('heading')).toBeNull();
  });

  it('says so without services, and shows an error of the server', async () => {
    stubApi({ 'GET homepage/prompage/data': { data: { data: null, options } } });
    const { unmount } = render(PromPage);
    expect(await screen.findByText('PromPage lists no services.')).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/prompage/data': {
        status: 401,
        message: 'PromPage answered with the status 404',
      },
    });
    render(PromPage);
    expect((await screen.findByRole('alert')).textContent).toContain(
      'PromPage answered with the status 404',
    );
  });
});

describe('EmbyLiveTv', () => {
  const numbers = { activeTimers: 2, seriesTimers: 1, todaysRecordings: 0, totalRecordings: 31 };
  const lists = {
    scheduledRecordings: [
      {
        date: 'tonight',
        name: 'Guests',
        seriesName: 'The Late Show',
        episodeTitle: 'Guests',
        channelName: 'Channel 12',
        userName: 'Zed',
        duration: '1h 30m',
        status: 'Scheduled',
      },
    ],
    completedRecordings: [
      { date: 'yesterday', name: 'The Film', seriesName: '', duration: '1h 30m' },
    ],
  };

  it('shows the numbers, the recordings to come and the recordings that are made', async () => {
    const sent = stubApi({
      'GET homepage/embyLiveTVTracker/stats': { data: numbers },
      'GET homepage/embyLiveTVTracker/activity': { data: lists },
    });
    render(EmbyLiveTv, { options: liveTvOptions({ showUserInfo: true }) });

    const scheduled = await screen.findByRole('table', { name: 'Scheduled recordings' });
    expect(text(scheduled)).toContain('tonight');
    expect(text(scheduled)).toContain('The Late Show');
    expect(text(scheduled)).toContain('Channel 12');
    expect(text(scheduled)).toContain('Zed');
    expect(text(scheduled)).toContain('1h 30m');
    expect(text(scheduled)).toContain('Scheduled');
    const completed = screen.getByRole('table', { name: 'Completed recordings' });
    expect(text(completed)).toContain('The Film');
    expect(await screen.findByText('Total Recordings')).toBeTruthy();
    expect(screen.getByText('31')).toBeTruthy();
    expect(sent.map((request) => request.path).sort()).toEqual([
      'homepage/embyLiveTVTracker/activity',
      'homepage/embyLiveTVTracker/stats',
    ]);
  });

  it('leaves out what the options switch off', async () => {
    stubApi({
      'GET homepage/embyLiveTVTracker/stats': { data: numbers },
      'GET homepage/embyLiveTVTracker/activity': { data: lists },
    });
    render(EmbyLiveTv, {
      options: liveTvOptions({
        compact: true,
        showDuration: false,
        showSeriesInfo: false,
        showCompleted: false,
      }),
    });

    const scheduled = await screen.findByRole('table', { name: 'Scheduled recordings' });
    expect(text(scheduled)).not.toContain('Zed');
    expect(text(scheduled)).not.toContain('1h 30m');
    expect(text(scheduled)).not.toContain('Episode title');
    expect(screen.queryByRole('table', { name: 'Completed recordings' })).toBeNull();
    expect(await screen.findByText('Series Timers')).toBeTruthy();
    expect(screen.queryByText('Total Recordings')).toBeNull();
  });

  it('says so without recordings, and shows an error of the server', async () => {
    stubApi({
      'GET homepage/embyLiveTVTracker/stats': { data: numbers },
      'GET homepage/embyLiveTVTracker/activity': {
        data: { scheduledRecordings: [], completedRecordings: [] },
      },
    });
    const { unmount } = render(EmbyLiveTv, { options: liveTvOptions(undefined) });
    expect(await screen.findByText('No scheduled recordings')).toBeTruthy();
    expect(screen.getByText('No completed recordings')).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/embyLiveTVTracker/stats': { status: 409, message: 'not enabled' },
      'GET homepage/embyLiveTVTracker/activity': { status: 403, message: 'Permission denied' },
    });
    render(EmbyLiveTv, { options: liveTvOptions(undefined) });
    const alerts = await screen.findAllByRole('alert');
    expect(alerts.map(text).join(' ')).toContain('Permission denied');
  });
});

describe('JellyStat', () => {
  const statistics = {
    period: '30 days',
    libraries: [
      { name: 'Films', type: 'Movies', item_count: 120, season_count: 0, episode_count: 0 },
    ],
    library_totals: { total_libraries: 1, total_items: 120, total_seasons: 0, total_episodes: 0 },
    users: [
      { name: 'robin', play_count: 3, last_activity: null },
      { name: 'pat', play_count: 1, last_activity: null },
    ],
    most_watched_movies: [{ id: '815', title: 'The Film', play_count: 2, year: '2019' }],
    most_watched_shows: [{ id: 'The Show', title: 'The Show', play_count: 1, year: null }],
    most_listened_music: [],
  };
  const url = 'https://stats.example.com';

  it('shows the libraries, the users and what was played most', async () => {
    stubApi({ 'GET homepage/jellystat': { data: statistics } });
    render(JellyStat, { options: jellyStatOptions({ url, maxItems: 1 }) });

    const libraries = await screen.findByRole('table', { name: 'Libraries of JellyStat' });
    expect(text(libraries)).toContain('Films');
    expect(text(libraries)).toContain('Movies');
    const users = screen.getByRole('table', { name: 'Users of JellyStat' });
    expect(text(users)).toContain('robin');
    // The list of the users is cut at the number of the settings.
    expect(text(users)).not.toContain('pat');
    expect(screen.getByText('30 days')).toBeTruthy();

    const films = screen.getByRole('list', { name: 'Most Watched Movies' });
    const poster = within(films).getByRole('button');
    expect(poster.getAttribute('title')).toContain('The Film');
    expect(poster.getAttribute('title')).toContain('2 plays');
    expect(poster.querySelector('img')?.getAttribute('src')).toBe(
      `${url}/proxy/Items/Images/Primary?id=815&fillWidth=200&quality=90`,
    );
    // A series that is known by its name only has no picture of JellyStat: it shows the
    // picture "Poster coming soon" of PHP.
    const series = screen.getByRole('list', { name: 'Most Watched TV Shows' });
    expect(within(series).getByRole('button').querySelector('img')?.getAttribute('src')).toBe(
      'plugins/images/homepage/no-list.png',
    );
    expect(screen.queryByRole('list', { name: 'Most Listened Music' })).toBeNull();
  });

  it('asks for the details of an item by its id', async () => {
    const sent = stubApi({
      'GET homepage/jellystat': { data: statistics },
      'POST homepage/jellystat/metadata': {
        data: {
          content: [
            {
              title: 'The Film',
              metadata: { summary: 'A film.', year: '2019' },
              address: 'https://media.example.com/item?id=815',
            },
          ],
        },
      },
    });
    render(JellyStat, { options: jellyStatOptions({ url }) });

    const films = await screen.findByRole('list', { name: 'Most Watched Movies' });
    await fireEvent.click(within(films).getByRole('button'));
    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(await within(dialog).findByText('A film.')).toBeTruthy();
    expect(sent.at(-1)).toEqual({
      method: 'POST',
      path: 'homepage/jellystat/metadata',
      body: { key: '815' },
    });
    const link = within(dialog).getByRole('link', {
      name: 'Open in the media server',
      hidden: true,
    });
    expect(link.getAttribute('href')).toBe('https://media.example.com/item?id=815');
  });

  it('leaves out what the options switch off, and says so without statistics', async () => {
    stubApi({ 'GET homepage/jellystat': { data: statistics } });
    const { unmount } = render(JellyStat, {
      options: jellyStatOptions({
        url,
        showLibraries: false,
        showUsers: false,
        showMovies: false,
      }),
    });
    expect(await screen.findByRole('list', { name: 'Most Watched TV Shows' })).toBeTruthy();
    expect(screen.queryByRole('table')).toBeNull();
    expect(screen.queryByRole('list', { name: 'Most Watched Movies' })).toBeNull();
    unmount();

    stubApi({
      'GET homepage/jellystat': {
        data: { period: '30 days', libraries: [], library_totals: [], users: [] },
      },
    });
    render(JellyStat, { options: jellyStatOptions({ url }) });
    expect(await screen.findByText(/JellyStat gave no statistics/)).toBeTruthy();
  });

  it('shows the pages of JellyStat in a frame, and asks the server for nothing', async () => {
    const sent = stubApi({});
    const { unmount } = render(JellyStat, {
      options: jellyStatOptions({ mode: 'iframe', url, frameHeight: 600, frameScrolling: false }),
    });

    const frame = screen.getByTitle('JellyStat');
    expect(frame.getAttribute('src')).toBe(url);
    expect(frame.getAttribute('height')).toBe('600');
    expect(frame.getAttribute('sandbox')).not.toContain('allow-top-navigation');
    expect(sent).toHaveLength(0);
    unmount();

    render(JellyStat, {
      options: jellyStatOptions({ mode: 'iframe', url: 'javascript:alert(1)' }),
    });
    expect(screen.queryByTitle('JellyStat')).toBeNull();
    expect(screen.getByText(/no web address/)).toBeTruthy();
  });

  it('shows an error of the server', async () => {
    stubApi({
      'GET homepage/jellystat': { status: 500, message: 'JellyStat URL or API key not configured' },
    });
    render(JellyStat, { options: jellyStatOptions({ url }) });

    expect((await screen.findByRole('alert')).textContent).toContain(
      'JellyStat URL or API key not configured',
    );
  });
});

describe('Donate', () => {
  const options = donateOptions({
    heading: 'Donate Please',
    description: 'Help me out <b>now</b>',
    minimum: 5,
    history: true,
  });

  it('links to the route that starts the payment, with the amount of the field', async () => {
    render(Donate, { options });

    const pay = screen.getByRole('link', { name: 'Donate $5.00' });
    expect(pay.getAttribute('href')).toMatch(/\/api\/v2\/homepage\/donate\/5$/);
    expect(pay.getAttribute('target')).toBe('_blank');
    // The text of the Admin shows as text.
    expect(screen.getByText('Help me out <b>now</b>')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Donate Please' })).toBeTruthy();

    const amount = screen.getByRole('spinbutton');
    await fireEvent.input(amount, { target: { value: '25' } });
    expect(screen.getByRole('link', { name: 'Donate $25.00' }).getAttribute('href')).toMatch(
      /\/homepage\/donate\/25$/,
    );
    // An amount below the minimum counts as the minimum, and cents are cut off.
    await fireEvent.input(amount, { target: { value: '2' } });
    expect(screen.getByRole('link', { name: 'Donate $5.00' })).toBeTruthy();
    await fireEvent.input(amount, { target: { value: '7.9' } });
    expect(screen.getByRole('link', { name: 'Donate $7.00' })).toBeTruthy();
  });

  it('lists the donations when the person asks for them', async () => {
    const sent = stubApi({
      'GET homepage/donate': {
        data: [{ date: 'last week', email: 'robin@example.com', amount: 12.5 }],
      },
    });
    render(Donate, { options });
    expect(sent).toHaveLength(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Show my donations' }));
    const table = await screen.findByRole('table', { name: 'Donations' });
    expect(text(table)).toContain('last week');
    expect(text(table)).toContain('robin@example.com');
    expect(text(table)).toContain('$12.50');

    await fireEvent.click(screen.getByRole('button', { name: 'Hide my donations' }));
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('says so without donations, shows an error, and has no list when it is off', async () => {
    stubApi({ 'GET homepage/donate': { data: [] } });
    const { unmount } = render(Donate, { options });
    await fireEvent.click(screen.getByRole('button', { name: 'Show my donations' }));
    expect(await screen.findByText('No donations yet.')).toBeTruthy();
    unmount();

    stubApi({ 'GET homepage/donate': { status: 500, message: 'Stripe cannot be reached' } });
    const failing = render(Donate, { options });
    await fireEvent.click(screen.getByRole('button', { name: 'Show my donations' }));
    expect((await screen.findByRole('alert')).textContent).toContain('Stripe cannot be reached');
    failing.unmount();

    render(Donate, { options: donateOptions({ heading: '', minimum: 1 }) });
    expect(screen.queryByRole('button', { name: 'Show my donations' })).toBeNull();
    expect(screen.getByRole('heading', { name: 'Donate' })).toBeTruthy();
  });
});

describe('Homepage', () => {
  it('shows the four last kinds of items with their options', async () => {
    stubApi({
      'GET homepage/items': {
        data: [
          { key: 'homepageOrderPromPage', kind: 'prompage', id: '' },
          {
            key: 'homepageOrderDonate',
            kind: 'donate',
            id: '',
            options: { heading: 'Help', description: '', minimum: 5, history: false },
          },
          {
            key: 'homepageOrderEmbyLiveTVTracker',
            kind: 'embylivetv',
            id: '',
            options: { showCompleted: false },
          },
          {
            key: 'homepageOrderJellyStat',
            kind: 'jellystat',
            id: '',
            options: { mode: 'iframe', url: 'https://stats.example.com' },
          },
        ],
      },
      'GET homepage/prompage/data': {
        data: {
          data: [{ name: 'Blog', status: true, uptime: 100 }],
          options: { title: 'Status', titleToggle: true, compact: true, showUptime: false },
        },
      },
      'GET homepage/embyLiveTVTracker/stats': { data: { activeTimers: 0 } },
      'GET homepage/embyLiveTVTracker/activity': {
        data: { scheduledRecordings: [], completedRecordings: [] },
      },
    });
    render(Homepage, { title: 'Homepage' });

    expect(await screen.findByRole('heading', { name: 'Status' })).toBeTruthy();
    expect(screen.getByRole('link', { name: 'Donate $5.00' })).toBeTruthy();
    expect(await screen.findByText('No scheduled recordings')).toBeTruthy();
    expect(screen.queryByText('No completed recordings')).toBeNull();
    expect(screen.getByTitle('JellyStat').getAttribute('src')).toBe('https://stats.example.com');
  });
});
