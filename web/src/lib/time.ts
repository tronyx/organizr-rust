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

/** A number of seconds as people say it: "45 s", "12 min", "1 h 2 min", "2 d 3 h". */
export function durationText(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return '';
  }
  const whole = Math.round(seconds);
  if (whole < 60) {
    return `${whole} s`;
  }
  const minutes = Math.floor(whole / 60);
  if (minutes < 60) {
    return `${minutes} min`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours} h ${minutes % 60} min`;
  }
  return `${Math.floor(hours / 24)} d ${hours % 24} h`;
}
