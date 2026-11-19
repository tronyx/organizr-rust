// Times as people say them.

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['day', 86400],
  ['hour', 3600],
  ['minute', 60],
];

/**
 * A time as people say it, for example "3 hours ago" or "in 2 days". `now` is the
 * present time in milliseconds.
 */
export function fromNow(time: string | null | undefined, now: number = Date.now()): string | null {
  if (!time) {
    return null;
  }
  const then = Date.parse(time);
  if (Number.isNaN(then)) {
    return null;
  }
  const seconds = Math.round((then - now) / 1000);
  const format = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
  for (const [unit, length] of UNITS) {
    if (Math.abs(seconds) >= length) {
      return format.format(Math.round(seconds / length), unit);
    }
  }
  return format.format(seconds, 'second');
}
