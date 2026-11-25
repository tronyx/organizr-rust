// What the settings pages share: the masked value of a secret, the tests of the value of a
// field, the list of the sites of UniFi, the note about the scripts, and the levels of the
// log. The forms themselves are the definitions in `src/settings/forms` (D-106).

/** What the server sends for a secret that has a value. The value itself stays there. */
export const SECURE = '***Secure***';

/** The tests that take the value of a field and not the stored settings. */
export type ValueTest = 'cron' | 'folder';

/** The body of `POST /test/<test>` for the value of a field, as PHP Organizr sends it. */
export function valueTestBody(test: ValueTest, value: string): unknown {
  return test === 'cron' ? value : { folder: value };
}

/** What the page says about the switch of the operator for the scripts of the Admin. */
export function scriptsNote(scriptsRun: boolean): string {
  return scriptsRun
    ? 'The scripts run now: the operator of the server set ORGANIZR_CUSTOM_JS=1.'
    : 'The scripts do not run now. They run only when the operator of the server sets the environment variable ORGANIZR_CUSTOM_JS=1.';
}

/**
 * The sites of a UniFi controller as a sentence, from the answer of `POST test/unifi/site`.
 * Nothing when the answer names no site.
 */
export function siteList(data: unknown): string | null {
  const sites = (data as { data?: unknown } | null)?.data;
  if (!Array.isArray(sites)) {
    return null;
  }
  const named = sites
    .filter(
      (site): site is { name: string; desc?: unknown } =>
        typeof site === 'object' && site !== null && typeof site.name === 'string',
    )
    .map((site) => (typeof site.desc === 'string' ? `${site.desc}: ${site.name}` : site.name));
  return named.length > 0 ? `The names of the sites are: ${named.join(', ')}` : null;
}

/** The levels of a record of the log, from the lowest, as PHP Organizr names them. */
export const LOG_LEVELS = [
  'DEBUG',
  'INFO',
  'NOTICE',
  'WARNING',
  'ERROR',
  'CRITICAL',
  'ALERT',
  'EMERGENCY',
] as const;
