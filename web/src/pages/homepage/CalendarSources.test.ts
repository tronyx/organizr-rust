import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { dayText, today } from '../../lib/calendar';
import { stubApi } from '../../test-helpers';
import Calendar from './Calendar.svelte';

const NOW = today();
const DAY = dayText(Number(NOW.slice(0, 4)), Number(NOW.slice(5, 7)), 10);

/** An episode as the server builds it for SickRage: a start with a space, and no link. */
const EPISODE = {
  id: 'Sick-0-Miss-1',
  title: 'The Show',
  start: `${DAY} 21:00:00`,
  className: 'inline-popups bg-calendar calendar-item tvID--4711',
  imagetype: 'tv text-danger',
  imagetypeFilter: 'tv',
  downloadFilter: 'text-danger',
  bgColor: 'bg-danger',
  details: {
    seasonCount: '',
    status: 'Continuing',
    topTitle: 'The Show',
    bottomTitle: 'S02E03 - The One',
    overview: 'What happens.',
    runtime: '',
    image: '/plugins/images/homepage/no-np.png',
    ratings: 'unknown',
    videoQuality: 'HD720p',
    genres: '',
  },
};

/** An event as the server builds it for a calendar in the iCal format. */
const DINNER = {
  title: 'Dinner',
  imagetype: 'calendar-o text-warning text-custom-calendar text-info',
  imagetypeFilter: 'ical',
  className: 'bg-calendar calendar-item bg-custom-calendar',
  start: DAY,
  end: DAY,
  bgColor: 'bg-info',
};

function show(ical: unknown) {
  stubApi({ 'GET homepage/calendar': { data: { ical, events: [EPISODE] } } });
  render(Calendar, { options: { defaultView: 'month', firstDay: 1, limit: 1000 } });
}

describe('Calendar with more sources', () => {
  it('shows the events of the iCal addresses on their days, with the others', async () => {
    show([DINNER]);

    const dinner = await screen.findByRole('button', { name: 'Dinner' });
    const cell = dinner.closest('td');
    expect(cell?.textContent).toContain('The Show');
    expect(dinner.getAttribute('title')).toBe('Dinner: Upcoming');

    // The kind and the states of these events have switches of their own.
    await fireEvent.click(screen.getByLabelText('Calendar events'));
    expect(screen.queryByRole('button', { name: 'Dinner' })).toBeNull();
    expect(screen.getByRole('button', { name: 'The Show' })).toBeTruthy();
    expect(screen.getByLabelText('Upcoming')).toBeTruthy();
  });

  it('opens an event of the iCal addresses, which has a title and a day only', async () => {
    show([DINNER]);
    await fireEvent.click(await screen.findByRole('button', { name: 'Dinner' }));

    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('Dinner');
    expect(dialog.textContent).toContain('Upcoming');
    expect(dialog.querySelector('img')).toBeNull();
    expect(dialog.querySelector('a')).toBeNull();
  });

  it('opens an episode of SickRage, which has no link, no rating and no genres', async () => {
    show(false);
    await fireEvent.click(await screen.findByRole('button', { name: 'The Show' }));

    const dialog = await screen.findByRole('dialog', { hidden: true });
    expect(dialog.textContent).toContain('S02E03 - The One');
    expect(dialog.textContent).toContain('What happens.');
    expect(dialog.textContent).toContain('Missing');
    expect(dialog.textContent).not.toContain('Rating');
    expect(dialog.querySelector('a')).toBeNull();
    expect(screen.queryByLabelText('Calendar events')).toBeNull();
  });
});
