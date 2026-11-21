import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { type CalendarEvent, dayText, today } from '../../lib/calendar';
import { stubApi } from '../../test-helpers';
import ArrQueue from './ArrQueue.svelte';
import Calendar from './Calendar.svelte';

const NOW = today();
const YEAR = Number(NOW.slice(0, 4));
const MONTH = Number(NOW.slice(5, 7));

function event(id: string, day: number, kind: string, state: string): CalendarEvent {
  return {
    id,
    title: id,
    start: dayText(YEAR, MONTH, day),
    imagetypeFilter: kind,
    downloadFilter: state,
    releaseType: kind === 'film' ? ['digital'] : undefined,
    details: {
      topTitle: id,
      bottomTitle: `About ${id}`,
      overview: `The story of ${id}.`,
      runtime: 45,
      image: '/plugins/images/homepage/no-np.png',
      ratings: 8.1,
      genres: ['Comedy'],
      href: 'https://sonarr.example.com/series/the-show',
      showLink: true,
    },
  };
}

const EVENTS = [
  event('The Show', 10, 'tv', 'text-success'),
  event('The Film', 10, 'film', 'text-danger'),
  event('The Album', 12, 'music', 'text-info'),
];

function show(options = { defaultView: 'month', firstDay: 1, limit: 1000 }) {
  stubApi({ 'GET homepage/calendar': { data: { ical: false, events: EVENTS } } });
  render(Calendar, { options });
}

describe('Calendar', () => {
  it('shows the events of the month on their days', async () => {
    show();

    const first = await screen.findByRole('button', { name: 'The Show' });
    const cell = first.closest('td');
    expect(cell).not.toBeNull();
    expect(within(cell as HTMLElement).getByRole('button', { name: 'The Film' })).toBeTruthy();
    expect(within(cell as HTMLElement).queryByRole('button', { name: 'The Album' })).toBeNull();
    expect(screen.getByRole('columnheader', { name: 'Mon' })).toBeTruthy();
  });

  it('shows no more events on a day than the limit, and says how many are left', async () => {
    show({ defaultView: 'month', firstDay: 1, limit: 1 });

    await screen.findByRole('button', { name: 'The Show' });
    expect(screen.queryByRole('button', { name: 'The Film' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: '1 more' }));
    // The list has every event of the day.
    expect(await screen.findByRole('button', { name: /The Film/ })).toBeTruthy();
  });

  it('leaves out the kinds and the states that are switched off', async () => {
    show();
    await screen.findByRole('button', { name: 'The Show' });

    await fireEvent.click(screen.getByLabelText('Films'));
    expect(screen.queryByRole('button', { name: 'The Film' })).toBeNull();
    expect(screen.getByRole('button', { name: 'The Show' })).toBeTruthy();

    await fireEvent.click(screen.getByLabelText('Not released yet'));
    expect(screen.queryByRole('button', { name: 'The Album' })).toBeNull();
  });

  it('starts in the week when the settings name it, with a column for each day', async () => {
    stubApi({
      'GET homepage/calendar': {
        data: {
          ical: false,
          events: [{ ...event('Tonight', 1, 'tv', 'text-success'), start: NOW }],
        },
      },
    });
    render(Calendar, { options: { defaultView: 'basicWeek', firstDay: 0, limit: 1000 } });

    const shown = await screen.findByRole('button', { name: /Tonight/ });
    expect(shown.textContent).toContain('About Tonight');
    expect(shown.textContent).toContain('Downloaded');
    expect(screen.getAllByRole('columnheader')).toHaveLength(7);
    expect(screen.getByRole('button', { name: 'Week' }).getAttribute('aria-pressed')).toBe('true');

    // The day view has the one day, and "Later" goes to the next day.
    await fireEvent.click(screen.getByRole('button', { name: 'Day' }));
    expect(screen.getAllByRole('columnheader')).toHaveLength(1);
    expect(screen.getByRole('button', { name: /Tonight/ })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Later' }));
    expect(screen.queryByRole('button', { name: /Tonight/ })).toBeNull();
  });

  it('starts in the list when the settings name the list', async () => {
    show({ defaultView: 'list', firstDay: 0, limit: 1000 });

    const row = await screen.findByRole('button', { name: /The Album/ });
    expect(row.textContent).toContain('About The Album');
    expect(row.textContent).toContain('Not released yet');
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('goes to another month and back to today', async () => {
    show({ defaultView: 'list', firstDay: 1, limit: 1000 });
    await screen.findByRole('button', { name: /The Show/ });
    const title = screen.getByRole('heading', { level: 2 }).textContent;

    await fireEvent.click(screen.getByRole('button', { name: 'Later' }));
    expect(screen.getByRole('heading', { level: 2 }).textContent).not.toBe(title);
    expect(screen.getByText('The calendar has nothing for this month.')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Today' }));
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe(title);
  });

  it('opens the details of an event', async () => {
    show();
    await fireEvent.click(await screen.findByRole('button', { name: 'The Film' }));

    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('The story of The Film.');
    expect(dialog.textContent).toContain('Missing');
    expect(dialog.textContent).toContain('digital');
    expect(dialog.textContent).toContain('45 minutes');
    // The picture that stands for "no picture" is not shown.
    expect(dialog.querySelector('img')).toBeNull();
    const link = within(dialog).getByRole('link', {
      name: 'Open in the application',
      hidden: true,
    });
    expect(link.getAttribute('href')).toBe('https://sonarr.example.com/series/the-show');
  });

  it('says why the events could not be loaded', async () => {
    stubApi({ 'GET homepage/calendar': { status: 500, message: 'No answer' } });
    render(Calendar, { options: { defaultView: 'month', firstDay: 1, limit: 10 } });

    expect((await screen.findByRole('alert')).textContent).toContain('No answer');
  });
});

describe('ArrQueue', () => {
  it('shows each download with its name and how much is done', async () => {
    stubApi({
      'GET homepage/sonarr/queue': {
        data: {
          content: {
            queueItems: [
              {
                title: 'The.Show.S02E03.1080p',
                status: 'downloading',
                size: 1000,
                sizeleft: 250,
                timeleft: '00:10:00',
                series: { title: 'The Show' },
                episode: { seasonNumber: 2, episodeNumber: 3, title: 'The One' },
              },
              { title: 'A.Release', status: 'queued', size: 0, sizeleft: 0 },
            ],
            historyItems: false,
          },
        },
      },
    });
    render(ArrQueue, { application: 'sonarr' });

    const table = await screen.findByRole('table', { name: 'Downloads of Sonarr' });
    const [, first, second] = within(table).getAllByRole('row') as [
      HTMLElement,
      HTMLElement,
      HTMLElement,
    ];
    expect(first.textContent).toContain('The Show S02E03');
    expect(first.textContent).toContain('downloading');
    expect(first.textContent).toContain('75 %');
    expect(first.textContent).toContain('00:10:00');
    expect(second.textContent).toContain('A.Release');
    expect(second.textContent).toContain('0 %');
  });

  it('says so when the queue is empty', async () => {
    stubApi({
      'GET homepage/radarr/queue': { data: { content: { queueItems: [], historyItems: false } } },
    });
    render(ArrQueue, { application: 'radarr' });

    expect(await screen.findByText('Nothing in queue')).toBeTruthy();
    expect(screen.getByRole('heading', { name: /Downloads of Radarr/ })).toBeTruthy();
  });
});
