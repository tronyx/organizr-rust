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

/**
 * The seconds of a time span as Sonarr, Radarr, Lidarr and Readarr give it (.NET
 * `TimeSpan`): "hh:mm:ss" with an optional fraction ("01:51:22.3339332") and an optional
 * day part ("2.03:04:05"). Null when the text is not such a span.
 */
export function timeSpanSeconds(text: string | null | undefined): number | null {
  const match = /^(?:(\d+)\.)?(\d{1,2}):(\d{2}):(\d{2})(?:\.\d+)?$/.exec(text?.trim() ?? '');
  if (!match) {
    return null;
  }
  const [, days, hours, minutes, seconds] = match;
  if (Number(minutes) > 59 || Number(seconds) > 59) {
    return null;
  }
  return Number(days ?? 0) * 86400 + Number(hours) * 3600 + Number(minutes) * 60 + Number(seconds);
}

/** A time span of the *arr applications as people say it ("1 h 51 min"), or "". */
export function timeSpanText(text: string | null | undefined): string {
  const seconds = timeSpanSeconds(text);
  return seconds === null ? '' : durationText(seconds);
}
