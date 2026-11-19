// The calendar of the homepage: its events, and the days of a month.

/** One event, as `GET homepage/calendar` describes it. */
export interface CalendarEvent {
  id: string;
  title: string;
  /** A day (`2026-11-20`) or a moment (`2026-11-20T20:00:00Z`). */
  start: string;
  /** `tv`, `film` or `music`. */
  imagetypeFilter: string;
  /** The state of the download, as a class name of the PHP interface. */
  downloadFilter: string;
  /** For a film: which releases are on this day. */
  releaseType?: string[];
  details: {
    topTitle: string;
    bottomTitle: string;
    overview: string | null;
    runtime: number | string | null;
    image: string;
    ratings: number | string | null;
    genres: string[] | null;
    href: string;
    showLink: boolean;
    videoQuality?: string | null;
    status?: string | null;
  };
}

/** How the calendar shows, from the settings of the install. */
export interface CalendarOptions {
  /** The view at the start: `month`, `basicWeek`, `basicDay` or `list`. */
  defaultView: string;
  /** The first day of a week: 0 for Sunday, 1 for Monday. */
  firstDay: number;
  /** The largest number of events that a day of the month view shows. */
  limit: number;
}

/** The kinds of events, with the word and the icon of each one. */
export const KINDS: Record<string, { word: string; icon: string }> = {
  tv: { word: 'Series', icon: 'fontawesome::tv' },
  film: { word: 'Films', icon: 'fontawesome::film' },
  music: { word: 'Music', icon: 'fontawesome::music' },
};

/** The states of a download, with the word and the colour of each one. */
export const STATES: Record<string, { word: string; badge: string }> = {
  'text-success': { word: 'Downloaded', badge: 'badge-success' },
  'text-info': { word: 'Not released yet', badge: 'badge-info' },
  'text-danger': { word: 'Missing', badge: 'badge-error' },
  'text-primary animated flash': { word: 'Premiere', badge: 'badge-primary' },
  'text-dark': { word: 'Not monitored', badge: 'badge-neutral' },
};

/** The state of an event. An unknown one shows as it came. */
export function stateOf(event: CalendarEvent): { word: string; badge: string } {
  return STATES[event.downloadFilter] ?? { word: event.downloadFilter, badge: 'badge-ghost' };
}

function pad(number: number): string {
  return String(number).padStart(2, '0');
}

/** A day as text: `2026-11-20`. `month` counts from 1. */
export function dayText(year: number, month: number, day: number): string {
  return `${year}-${pad(month)}-${pad(day)}`;
}

/**
 * The day of an event in the time zone of the browser. A start without a time is that
 * day everywhere.
 */
export function dayOf(start: string): string {
  if (/^\d{4}-\d{2}-\d{2}$/.test(start)) {
    return start;
  }
  const moment = new Date(start);
  if (Number.isNaN(moment.getTime())) {
    return start.slice(0, 10);
  }
  return dayText(moment.getFullYear(), moment.getMonth() + 1, moment.getDate());
}

/** The time of an event in the time zone of the browser, or nothing for a start without a time. */
export function timeOf(start: string): string | null {
  if (!start.includes('T')) {
    return null;
  }
  const moment = new Date(start);
  if (Number.isNaN(moment.getTime())) {
    return null;
  }
  return `${pad(moment.getHours())}:${pad(moment.getMinutes())}`;
}

/** Today, as a day of the time zone of the browser. `now` is in milliseconds. */
export function today(now: number = Date.now()): string {
  const moment = new Date(now);
  return dayText(moment.getFullYear(), moment.getMonth() + 1, moment.getDate());
}

/** One day of the month view. */
export interface GridDay {
  /** The day as text. */
  day: string;
  /** The number of the day in its month. */
  number: number;
  /** Whether the day belongs to the month that the view shows. */
  inMonth: boolean;
}

/**
 * The weeks of a month, each with seven days. The first and the last week are filled
 * with days of the months around. `month` counts from 1, and `firstDay` is 0 for a
 * week that starts on Sunday.
 */
export function monthGrid(year: number, month: number, firstDay: number): GridDay[][] {
  const first = new Date(year, month - 1, 1);
  const before = (first.getDay() - (firstDay % 7) + 7) % 7;
  const days = new Date(year, month, 0).getDate();
  const weeks: GridDay[][] = [];
  for (let offset = -before; offset < days; offset += 7) {
    const week: GridDay[] = [];
    for (let place = 0; place < 7; place += 1) {
      const date = new Date(year, month - 1, 1 + offset + place);
      week.push({
        day: dayText(date.getFullYear(), date.getMonth() + 1, date.getDate()),
        number: date.getDate(),
        inMonth: date.getMonth() === month - 1,
      });
    }
    weeks.push(week);
  }
  return weeks;
}

/** The short names of the days of a week that starts on `firstDay`. */
export function weekDays(firstDay: number): string[] {
  const names = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  return names.map((_, place) => names[(place + firstDay) % 7] ?? '');
}

/** The name of a month with its year: "November 2026". `month` counts from 1. */
export function monthName(year: number, month: number): string {
  const names = [
    'January',
    'February',
    'March',
    'April',
    'May',
    'June',
    'July',
    'August',
    'September',
    'October',
    'November',
    'December',
  ];
  return `${names[month - 1] ?? ''} ${year}`;
}

/** The month before or after: `step` is -1 or 1. */
export function moveMonth(
  year: number,
  month: number,
  step: number,
): { year: number; month: number } {
  const moved = new Date(year, month - 1 + step, 1);
  return { year: moved.getFullYear(), month: moved.getMonth() + 1 };
}

/** The events of each day, in the order of their start. */
export function eventsByDay(events: CalendarEvent[]): Map<string, CalendarEvent[]> {
  const sorted = [...events].sort((a, b) => a.start.localeCompare(b.start));
  const byDay = new Map<string, CalendarEvent[]>();
  for (const event of sorted) {
    const day = dayOf(event.start);
    const list = byDay.get(day);
    if (list) {
      list.push(event);
    } else {
      byDay.set(day, [event]);
    }
  }
  return byDay;
}

/** The events of the kinds and the states that are not switched off. */
export function shownEvents(
  events: CalendarEvent[],
  hiddenKinds: ReadonlySet<string>,
  hiddenStates: ReadonlySet<string>,
): CalendarEvent[] {
  return events.filter(
    (event) => !hiddenKinds.has(event.imagetypeFilter) && !hiddenStates.has(event.downloadFilter),
  );
}

/** A day as people read it: "Fri, 20 November 2026". */
export function longDay(day: string): string {
  const [year, month, number] = day.split('-').map(Number);
  if (!year || !month || !number) {
    return day;
  }
  const date = new Date(year, month - 1, number);
  const name = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][date.getDay()] ?? '';
  return `${name}, ${number} ${monthName(year, month)}`;
}
