// The log of the server, as `GET /log` sends it: records in pages, from the log file of
// a day or from all of them.

/** One record of the log, with the field names of PHP Organizr. */
export interface LogRecord {
  log_level: string;
  message: string;
  /** The part of the server that wrote the record. PHP calls it the function. */
  channel: string;
  username: string;
  trace_id: string;
  /** The id of the request that wrote the record: the header `X-Request-Id` of its answer. */
  request_id: string | null;
  file: string;
  line: number | null;
  context: Record<string, unknown>;
  remote_ip_address: string | null;
  /** The time in UTC, as `2026-11-20 03:15:00.123456`. */
  datetime: string;
  timezone: string;
}

/** A page of records. `false` when the server has no log file. */
export interface LogPage {
  pageInfo: { results: number; totalPages: number; pageSize: number | string; page: number };
  /** `null` stands for a line of the file that is no record. */
  results: (LogRecord | null)[];
  /** The days of the log files, the newest first. Their places are the numbers of the files. */
  files?: string[];
}

/**
 * The path of a page of the log: `file` is the place of a file on the list, or `all`, and
 * `filter` is a level or `NONE`.
 */
export function logPath(file: string, filter: string, offset: number, pageSize: number): string {
  const query = new URLSearchParams({
    filter,
    pageSize: String(pageSize),
    offset: String(offset),
  });
  return `log/${encodeURIComponent(file)}?${query.toString()}`;
}

/** The path of the record with a trace id, searched in all the files. */
export function tracePath(traceId: string): string {
  return `log/all/${encodeURIComponent(traceId)}`;
}

/** The color of the badge of a level. */
export function levelClass(level: string): string {
  switch (level.toUpperCase()) {
    case 'DEBUG':
      return 'badge-ghost';
    case 'INFO':
      return 'badge-info';
    case 'NOTICE':
      return 'badge-neutral';
    case 'WARNING':
      return 'badge-warning';
    default:
      return 'badge-error';
  }
}

/** The time of a record as `2026-11-20 03:15:00 UTC`. */
export function timeText(record: LogRecord): string {
  return `${record.datetime.slice(0, 19)} ${record.timezone || 'UTC'}`;
}
