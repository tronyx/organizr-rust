// The last items of the homepage: the options that the server sends with the items of
// JellyStat, of the live television of Emby and of the donations, and small helpers.

/** A value of the options of an item as a whole number, or the given one. */
function numberOf(value: unknown, otherwise: number): number {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? Math.round(number) : otherwise;
}

/** A value of the options of an item as a switch. Without a value, the given one counts. */
function switchOf(value: unknown, otherwise: boolean): boolean {
  return typeof value === 'boolean' ? value : otherwise;
}

/** How the JellyStat item shows. */
export interface JellyStatOptions {
  /** `native` for the statistics, `iframe` for the pages of JellyStat in a frame. */
  mode: 'native' | 'iframe';
  /** The address of JellyStat for the browser. */
  url: string;
  frameHeight: number;
  frameScrolling: boolean;
  /** The minutes between two loads. */
  refreshMinutes: number;
  days: number;
  maxItems: number;
  showLibraries: boolean;
  showUsers: boolean;
  showMostWatched: boolean;
  showMovies: boolean;
  showShows: boolean;
  showMusic: boolean;
}

/** The options of the JellyStat item, from what `GET homepage/items` sends. */
export function jellyStatOptions(options: Record<string, unknown> | undefined): JellyStatOptions {
  const sent = options ?? {};
  return {
    mode: sent.mode === 'iframe' ? 'iframe' : 'native',
    url: typeof sent.url === 'string' ? sent.url : '',
    frameHeight: numberOf(sent.frameHeight, 800),
    frameScrolling: switchOf(sent.frameScrolling, true),
    refreshMinutes: numberOf(sent.refreshMinutes, 5),
    days: numberOf(sent.days, 30),
    maxItems: numberOf(sent.maxItems, 10),
    showLibraries: switchOf(sent.showLibraries, true),
    showUsers: switchOf(sent.showUsers, true),
    showMostWatched: switchOf(sent.showMostWatched, true),
    showMovies: switchOf(sent.showMovies, true),
    showShows: switchOf(sent.showShows, true),
    showMusic: switchOf(sent.showMusic, true),
  };
}

/** How the live television item shows. */
export interface LiveTvOptions {
  /** The minutes between two loads. */
  refreshMinutes: number;
  compact: boolean;
  showDuration: boolean;
  showSeriesInfo: boolean;
  showUserInfo: boolean;
  showCompleted: boolean;
}

/** The options of the live television item, from what `GET homepage/items` sends. */
export function liveTvOptions(options: Record<string, unknown> | undefined): LiveTvOptions {
  const sent = options ?? {};
  return {
    refreshMinutes: numberOf(sent.refreshMinutes, 5),
    compact: switchOf(sent.compact, false),
    showDuration: switchOf(sent.showDuration, true),
    showSeriesInfo: switchOf(sent.showSeriesInfo, true),
    showUserInfo: switchOf(sent.showUserInfo, false),
    showCompleted: switchOf(sent.showCompleted, true),
  };
}

/** How the donate item shows. */
export interface DonateOptions {
  heading: string;
  description: string;
  /** The smallest amount, in dollars. */
  minimum: number;
  /** Whether the item offers the list of the donations. */
  history: boolean;
}

/** The options of the donate item, from what `GET homepage/items` sends. */
export function donateOptions(options: Record<string, unknown> | undefined): DonateOptions {
  const sent = options ?? {};
  const minimum = Number(sent.minimum);
  return {
    heading: typeof sent.heading === 'string' ? sent.heading : '',
    description: typeof sent.description === 'string' ? sent.description : '',
    minimum: Number.isFinite(minimum) && minimum > 0 ? minimum : 1,
    history: sent.history === true,
  };
}

/**
 * A moment as the date and the time of the browser. A text that is no moment stays as
 * it is.
 */
export function momentText(moment: string | null | undefined): string {
  if (!moment) {
    return '';
  }
  const time = Date.parse(moment);
  if (Number.isNaN(time)) {
    return moment;
  }
  return new Date(time).toLocaleString('en', { dateStyle: 'medium', timeStyle: 'short' });
}

/** An amount of money in US dollars, for example `$12.50`. */
export function dollars(amount: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(amount);
}

/**
 * The address of the poster of an item at JellyStat, which passes the picture of the
 * media server on. Nothing without an address of JellyStat or without an id.
 */
export function jellyStatPoster(url: string, id: unknown): string | null {
  const text = typeof id === 'string' || typeof id === 'number' ? String(id) : '';
  if (!url || !/^[A-Za-z0-9-]+$/.test(text)) {
    return null;
  }
  const base = url.replace(/\/+$/, '');
  return `${base}/proxy/Items/Images/Primary?id=${encodeURIComponent(text)}&fillWidth=200&quality=90`;
}

/** The address of a frame, when it is a web address or a path of this site. */
export function frameAddress(url: string): string | null {
  return /^https?:\/\//i.test(url) || url.startsWith('/') ? url : null;
}
