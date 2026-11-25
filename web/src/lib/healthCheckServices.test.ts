import { describe, expect, it } from 'vitest';

import { emptyRow, fromSetting, importChecks, toSetting } from './healthCheckServices';

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
  '2': [
    { label: 'Service Name', value: 'Radarr' },
    { label: 'UUID', value: 'third-id' },
    { label: 'Enabled', value: true },
    { label: 'Send API Key', value: false },
  ],
  '3': 'not a service',
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
        sendKey: true,
      },
      {
        name: 'Sonarr',
        uuid: 'another-id',
        external: '',
        internal: '',
        enabled: true,
        sendKey: true,
      },
      {
        name: 'Radarr',
        uuid: 'third-id',
        external: '',
        internal: '',
        enabled: true,
        sendKey: false,
      },
    ]);
  });

  it('sends the API key to a service of PHP Organizr, which has no switch for it', () => {
    expect(fromSetting(STORED).map((row) => row.sendKey)).toEqual([true, true, false]);
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
        sendKey: true,
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
    expect(setting['1']?.[5]).toEqual({
      label: 'Send API Key',
      name: 'HEALTHCHECKS-all-items[1].sendKey',
      type: 'switch',
      value: false,
    });
    expect(fromSetting(setting)).toEqual([{ ...rows[0], name: 'Plex' }, rows[1]]);
  });

  it('writes the empty text for no services', () => {
    expect(toSetting([])).toBe('');
  });
});

describe('importChecks', () => {
  const ID_A = '0b7c9f3e-1a2b-4c3d-8e9f-001122334455';
  const ID_B = '7f1e2d3c-4b5a-4968-8776-aabbccddeeff';

  it('adds the checks that the list does not have, by the id in the ping address', () => {
    const rows = [{ ...emptyRow(), name: 'Plex', uuid: ID_A.toUpperCase() }];
    const checks = [
      { name: 'Plex again', ping_url: `https://hc-ping.com/${ID_A}` },
      { name: 'Sonarr', ping_url: `https://hc-ping.com/${ID_B}` },
      { name: 'Sonarr twice', ping_url: `https://hc-ping.com/${ID_B}` },
      { name: 'No address' },
      'not a check',
    ];

    const imported = importChecks(rows, checks);

    expect(imported.readOnly).toBe(false);
    expect(imported.added).toEqual(['Sonarr']);
    expect(imported.rows).toEqual([
      rows[0],
      // An imported check gets no API key.
      {
        name: 'Sonarr',
        uuid: ID_B,
        external: '',
        internal: '',
        enabled: true,
        sendKey: false,
      },
    ]);
    // The list that was handed in stays as it was.
    expect(rows).toHaveLength(1);
  });

  it('says so when the key of Healthchecks can only read, and adds nothing', () => {
    const rows = [emptyRow()];
    const imported = importChecks(rows, [{ name: 'Plex', unique_key: 'abc' }]);
    expect(imported).toEqual({ rows, added: [], readOnly: true });
  });

  it('adds nothing for an answer without checks', () => {
    expect(importChecks([], false).added).toEqual([]);
    expect(importChecks([], null).rows).toEqual([]);
  });
});
