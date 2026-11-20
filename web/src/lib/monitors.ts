// Helpers of the homepage items that show a search or the state of a service.

/** An address that a link may open: one of the web, or a magnet link when allowed. */
export function safeLink(address: unknown, magnet = false): string | null {
  if (typeof address !== 'string') {
    return null;
  }
  if (/^https?:\/\//i.test(address) || (magnet && /^magnet:\?/i.test(address))) {
    return address;
  }
  return null;
}

/** A number of any kind of value, or nothing when the value is no number. */
export function numberOf(value: unknown): number | null {
  if (value === null || value === undefined || value === '' || typeof value === 'boolean') {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

/** A whole number with commas between the thousands. */
export function grouped(value: unknown): string {
  return Math.round(numberOf(value) ?? 0).toLocaleString('en-US');
}

/** A number with a fixed count of digits after the point, or a dash for no number. */
export function fixed(value: unknown, digits = 1): string {
  const number = numberOf(value);
  return number === null ? '–' : number.toFixed(digits);
}

const SIZES = ['KiB', 'MiB', 'GiB', 'TiB', 'PiB'];

/** A number of bytes as people say it, in steps of 1024, as PHP Organizr shows it. */
export function fileSize(bytes: unknown): string {
  let size = numberOf(bytes);
  if (size === null) {
    return '';
  }
  if (Math.abs(size) < 1024) {
    return `${size} B`;
  }
  let unit = -1;
  do {
    size /= 1024;
    unit += 1;
  } while (Math.abs(size) >= 1024 && unit < SIZES.length - 1);
  return `${size.toFixed(1)} ${SIZES[unit]}`;
}

function counted(count: number, unit: string): string {
  return `${count} ${unit}${count > 1 ? 's' : ''}`;
}

/** A time of a print job in words, for example "1 hour 30 minutes". */
export function printTime(seconds: unknown): string {
  const whole = Math.floor(numberOf(seconds) ?? 0);
  if (whole <= 0) {
    return '';
  }
  const parts: [number, string][] = [
    [Math.floor(whole / 86400), 'day'],
    [Math.floor(whole / 3600) % 24, 'hour'],
    [Math.floor(whole / 60) % 60, 'minute'],
    [whole % 60, 'second'],
  ];
  return parts
    .filter(([count]) => count > 0)
    .map(([count, unit]) => counted(count, unit))
    .join(' ');
}

/** A part of 100, kept between 0 and 100. */
export function partOf(percent: unknown): number {
  return Math.min(Math.max(numberOf(percent) ?? 0, 0), 100);
}
