import { describe, expect, it } from 'vitest';

import {
  allEvents,
  type CalendarEvent,
  dayOf,
  genresOf,
  KINDS,
  shortTime,
  stateOf,
  timeOf,
} from './calendar';

const EPISODE: CalendarEvent = {
  id: 'Sick-0-Miss-1',
  title: 'The Show',
  start: '2020-01-05 21:00:00',
  imagetypeFilter: 'tv',
  downloadFilter: 'text-danger',
  details: {
    topTitle: 'The Show',
    bottomTitle: 'S02E03 - The One',
    overview: '',
    runtime: '',
    image: '/plugins/images/homepage/no-np.png',
    ratings: '',
    genres: '',
  },
};

describe('allEvents', () => {
  it('gives the events of the iCal addresses the form of the others', () => {
    const events = allEvents({
      events: [EPISODE],
      ical: [
        {
          title: 'Dinner',
          start: '2020-01-20T18:00:00+01:00',
          end: '2020-01-20T19:30:00+01:00',
          bgColor: 'bg-info',
        },
        { title: 'A birthday', start: '2020-01-10', end: '2020-01-11', bgColor: 'bg-success' },
      ],
    });

    expect(events.map((event) => event.id)).toEqual(['Sick-0-Miss-1', 'ical-0', 'ical-1']);
    const dinner = events[1] as CalendarEvent;
    expect(dinner.imagetypeFilter).toBe('ical');
    expect(dinner.details.topTitle).toBe('Dinner');
    expect(dinner.end).toBe('2020-01-20T19:30:00+01:00');
    expect(stateOf(dinner).word).toBe('Upcoming');
    expect(stateOf(events[2] as CalendarEvent).word).toBe('Over');
    expect(KINDS[dinner.imagetypeFilter]?.word).toBe('Calendar events');
  });

  it('has the other events only for a person who does not get the iCal addresses', () => {
    expect(allEvents({ events: [EPISODE], ical: false })).toEqual([EPISODE]);
    expect(allEvents({ events: null, ical: null })).toEqual([]);
    expect(allEvents(null)).toEqual([]);
  });
});

describe('events of SickRage', () => {
  it('have a start with a space, which is a time of the browser', () => {
    expect(dayOf(EPISODE.start)).toBe('2020-01-05');
    expect(timeOf(EPISODE.start)).toBe('21:00');
    expect(shortTime(EPISODE.start)).toBe('9:00p');
    // The time of a download has no seconds.
    expect(dayOf('2020-01-06 22:10')).toBe('2020-01-06');
    expect(timeOf('2020-01-06 22:10')).toBe('22:10');
    expect(timeOf('2020-01-06')).toBeNull();
  });

  it('have no genres, where other events have a list', () => {
    expect(genresOf(EPISODE)).toEqual([]);
    expect(genresOf({ ...EPISODE, details: { ...EPISODE.details, genres: ['Drama'] } })).toEqual([
      'Drama',
    ]);
  });
});

describe('events of Trakt', () => {
  const FILM: CalendarEvent = {
    id: 'Trakt-Movie-1',
    title: 'The Film',
    start: '2099-01-07',
    imagetypeFilter: 'film',
    downloadFilter: 'text-dark',
    details: {
      topTitle: 'The Film',
      bottomTitle: 'One more time',
      overview: 'A film.',
      runtime: 110,
      image: '/plugins/images/homepage/no-np.png',
      ratings: 7.5,
      genres: ['action'],
    },
  };
  const PREMIERE: CalendarEvent = {
    ...EPISODE,
    id: 'Trakt-Tv-1',
    start: '2099-01-05T02:00:00.000Z',
    downloadFilter: 'text-primary animated flash',
  };

  it('have the states that Trakt can know: to come, premiere, and aired', () => {
    expect(stateOf(FILM).word).toBe('Not monitored');
    expect(stateOf(PREMIERE).word).toBe('Premiere');
    expect(stateOf({ ...PREMIERE, downloadFilter: 'text-danger' }).word).toBe('Missing');
    expect(KINDS[FILM.imagetypeFilter]?.word).toBe('Films');
  });

  it('stand on a day for a film, and at a moment for an episode', () => {
    expect(dayOf(FILM.start)).toBe('2099-01-07');
    expect(timeOf(FILM.start)).toBeNull();
    expect(timeOf(PREMIERE.start)).not.toBeNull();
    expect(genresOf(FILM)).toEqual(['action']);
    expect(allEvents({ events: [FILM, PREMIERE], ical: false })).toHaveLength(2);
  });
});
