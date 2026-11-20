import { describe, expect, it } from 'vitest';

import { levelClass, logPath, tracePath } from './logs';
import { LOGS } from './settingsForm';

describe('the log', () => {
  it('asks for a page of a file with the query of PHP Organizr', () => {
    expect(logPath('0', 'NONE', 0, 50)).toBe('log/0?filter=NONE&pageSize=50&offset=0');
    expect(logPath('all', 'WARNING', 100, 25)).toBe(
      'log/all?filter=WARNING&pageSize=25&offset=100',
    );
    expect(tracePath('a b/c')).toBe('log/all/a%20b%2Fc');
  });

  it('gives each level a colour', () => {
    expect(levelClass('INFO')).toBe('badge-info');
    expect(levelClass('warning')).toBe('badge-warning');
    expect(levelClass('EMERGENCY')).toBe('badge-error');
  });

  it('has the settings of the log of PHP Organizr, without its folder', () => {
    const fields = LOGS.flatMap((group) => group.fields);
    expect(fields.map((field) => [field.key, field.kind])).toEqual([
      ['logLevel', 'select'],
      ['maxLogFiles', 'number'],
      ['logLiveUpdateRefresh', 'select'],
      ['logPageSize', 'select'],
    ]);
    const level = fields[0];
    expect(level?.kind === 'select' && level.options.map((option) => option.value)).toEqual([
      'DEBUG',
      'INFO',
      'NOTICE',
      'WARNING',
      'ERROR',
      'CRITICAL',
      'ALERT',
      'EMERGENCY',
    ]);
  });
});
