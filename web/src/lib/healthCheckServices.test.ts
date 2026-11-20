import { describe, expect, it } from 'vitest';

import { emptyRow, fromSetting, toSetting } from './healthCheckServices';

const STORED = {
  '0': {
    '0': {
      label: 'Service Name',
      name: 'HEALTHCHECKS-all-items[36].name',
      type: 'input',
      value: 'Plex',
    },
    '1': {
      label: 'UUID',
      name: 'HEALTHCHECKS-all-items[36].uuid',
      type: 'input',
      value: 'made-up-id',
    },
    '2': { label: 'External URL', type: 'input', value: 'https://plex.example.com' },
    '3': { label: 'Internal URL', type: 'input', value: '' },
    '4': { label: 'Enabled', type: 'switch', value: false },
  },
  '1': [
    { label: 'Service Name', value: 'Sonarr' },
    { label: 'UUID', value: 'another-id' },
    { label: 'Enabled', value: 'true' },
  ],
  '2': 'not a service',
};

describe('fromSetting', () => {
  it('reads services that are stored as lists or as maps', () => {
    expect(fromSetting(STORED)).toEqual([
      {
        name: 'Plex',
        uuid: 'made-up-id',
        external: 'https://plex.example.com',
        internal: '',
        enabled: false,
      },
      { name: 'Sonarr', uuid: 'another-id', external: '', internal: '', enabled: true },
    ]);
  });

  it('reads no services from an empty setting', () => {
    expect(fromSetting('')).toEqual([]);
    expect(fromSetting(null)).toEqual([]);
    expect(fromSetting([])).toEqual([]);
  });
});

describe('toSetting', () => {
  it('writes the form of PHP Organizr, which reads back as the same services', () => {
    const rows = [
      {
        name: ' Plex ',
        uuid: 'made-up-id',
        external: 'https://plex.example.com',
        internal: '',
        enabled: true,
      },
      { ...emptyRow(), name: 'Sonarr', enabled: false },
    ];

    const setting = toSetting(rows) as Record<
      string,
      { label: string; name: string; type: string; value: unknown }[]
    >;

    expect(Object.keys(setting)).toEqual(['0', '1']);
    expect(setting['0']?.[0]).toEqual({
      label: 'Service Name',
      name: 'HEALTHCHECKS-all-items[0].name',
      type: 'input',
      value: 'Plex',
    });
    expect(setting['1']?.[4]).toEqual({
      label: 'Enabled',
      name: 'HEALTHCHECKS-all-items[1].enabled',
      type: 'switch',
      value: false,
    });
    expect(fromSetting(setting)).toEqual([{ ...rows[0], name: 'Plex' }, rows[1]]);
  });

  it('writes the empty text for no services', () => {
    expect(toSetting([])).toBe('');
  });
});
