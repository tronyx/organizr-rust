import { describe, expect, it } from 'vitest';

import {
  type CalendarEvent,
  dayOf,
  eventsByDay,
  longDay,
  monthGrid,
  monthName,
  moveMonth,
  shownEvents,
  stateOf,
  timeOf,
  weekDays,
} from './calendar';

function event(id: string, start: string, kind = 'tv', state = 'text-success'): CalendarEvent {
  return {
    id,
    title: id,
    start,
    imagetypeFilter: kind,
    downloadFilter: state,
    details: {
      topTitle: id,
      bottomTitle: '',
      overview: '',
      runtime: 45,
      image: '',
      ratings: 8,
      genres: [],
      href: '',
      showLink: false,
    },
  };
}

describe('monthGrid', () => {
  it('fills the weeks of a month with the days around it', () => {
    // November 2026 starts on a Sunday.
    const weeks = monthGrid(2026, 11, 1);

    expect(weeks).toHaveLength(6);
    expect(weeks.every((week) => week.length === 7)).toBe(true);
    expect(weeks[0]?.[0]).toEqual({ day: '2026-10-26', number: 26, inMonth: false });
    expect(weeks[0]?.[6]).toEqual({ day: '2026-11-01', number: 1, inMonth: true });
    expect(weeks[5]?.[0]).toEqual({ day: '2026-11-30', number: 30, inMonth: true });
    expect(weeks[5]?.[6]).toEqual({ day: '2026-12-06', number: 6, inMonth: false });
  });

  it('starts the week on the day of the settings', () => {
    const weeks = monthGrid(2026, 11, 0);

    expect(weeks).toHaveLength(5);
    expect(weeks[0]?.[0]?.day).toBe('2026-11-01');
    expect(weeks[4]?.[6]?.day).toBe('2026-12-05');
    expect(weekDays(0)[0]).toBe('Sun');
    expect(weekDays(1)).toEqual(['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']);
  });

  it('has four weeks for a February that fits', () => {
    // February 2027 starts on a Monday and has 28 days.
    expect(monthGrid(2027, 2, 1)).toHaveLength(4);
  });
});

describe('months', () => {
  it('moves over the end of a year', () => {
    expect(moveMonth(2026, 12, 1)).toEqual({ year: 2027, month: 1 });
    expect(moveMonth(2026, 1, -1)).toEqual({ year: 2025, month: 12 });
    expect(monthName(2026, 11)).toBe('November 2026');
    expect(longDay('2026-11-20')).toBe('Fri, 20 November 2026');
  });
});

describe('events', () => {
  it('have the day of their start, and a day without a time is that day', () => {
    expect(dayOf('2026-11-20')).toBe('2026-11-20');
    expect(timeOf('2026-11-20')).toBeNull();
    // Noon in UTC is the same day in the time zones that tests run in.
    expect(dayOf('2026-11-20T12:00:00Z')).toBe('2026-11-20');
    expect(timeOf('2026-11-20T12:00:00Z')).toMatch(/^\d\d:\d\d$/);
    expect(dayOf('soon')).toBe('soon');
  });

  it('are grouped by day, in the order of their start', () => {
    const byDay = eventsByDay([
      event('late', '2026-11-20T12:30:00Z'),
      event('film', '2026-11-21'),
      event('early', '2026-11-20T12:00:00Z'),
    ]);

    expect([...byDay.keys()]).toEqual(['2026-11-20', '2026-11-21']);
    expect(byDay.get('2026-11-20')?.map((one) => one.id)).toEqual(['early', 'late']);
  });

  it('are left out when their kind or their state is switched off', () => {
    const events = [
      event('a', '2026-11-20', 'tv', 'text-success'),
      event('b', '2026-11-20', 'film', 'text-danger'),
      event('c', '2026-11-20', 'music', 'text-success'),
    ];

    const shown = shownEvents(events, new Set(['music']), new Set(['text-danger']));

    expect(shown.map((one) => one.id)).toEqual(['a']);
  });

  it('have a word for their state', () => {
    expect(stateOf(event('a', '2026-11-20', 'tv', 'text-danger')).word).toBe('Missing');
    expect(stateOf(event('a', '2026-11-20', 'tv', 'text-new')).word).toBe('text-new');
  });
});
