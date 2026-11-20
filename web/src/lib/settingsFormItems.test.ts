import { describe, expect, it } from 'vitest';

import {
  BACKUPS,
  type Field,
  GENERAL,
  HEALTH_CHECKS_PLUGIN,
  HOMEPAGE,
  siteList,
  valueTestBody,
} from './settingsForm';

const FIELDS = HOMEPAGE.flatMap((group) => group.fields);

function field(key: string): Field | undefined {
  return FIELDS.find((candidate) => candidate.key === key);
}

function group(title: string) {
  return HOMEPAGE.find((candidate) => candidate.title === title);
}

describe('the settings of the downloaders', () => {
  it('have a switch that puts each one into the box of the downloaders', () => {
    const switches: [string, string][] = [
      ['sabnzbdCombine', 'SABnzbd'],
      ['nzbgetCombine', 'NZBGet'],
      ['transmissionCombine', 'Transmission'],
      ['qBittorrentCombine', 'qBittorrent'],
      ['delugeCombine', 'Deluge'],
      ['homepageSonarrQueueCombine', 'the downloads of Sonarr'],
      ['homepageRadarrQueueCombine', 'the downloads of Radarr'],
    ];
    for (const [key, name] of switches) {
      expect(field(key), key).toEqual({
        key,
        label: `Show ${name} in the box of the downloaders`,
        kind: 'switch',
      });
      expect(FIELDS.filter((candidate) => candidate.key === key)).toHaveLength(1);
    }
  });

  it('have the switch in the group of its downloader', () => {
    const keys = (title: string) => group(title)?.fields.map((one) => one.key) ?? [];

    expect(keys('SABnzbd')).toContain('sabnzbdCombine');
    expect(keys('NZBGet')).toContain('nzbgetCombine');
    expect(keys('Transmission')).toContain('transmissionCombine');
    expect(keys('qBittorrent')).toContain('qBittorrentCombine');
    expect(keys('Sonarr')).toContain('homepageSonarrQueueCombine');
    expect(keys('Radarr')).toContain('homepageRadarrQueueCombine');
  });
});

describe('the settings of Ombi', () => {
  it('have a connection test, and keep the API key a secret', () => {
    const ombi = group('Ombi');

    expect(ombi?.test).toBe('ombi');
    expect(field('ombiToken')?.kind).toBe('secret');
    expect(ombi?.fields.map((one) => one.key)).toEqual([
      'homepageOmbiEnabled',
      'homepageOmbiAuth',
      'homepageOrderombi',
      'ombiToken',
      'ombiDisableCertCheck',
      'ombiLimitUser',
      'ombiLimit',
      'homepageOmbiRequestAuth',
      'ombiTvDefault',
      'ombiAlias',
      'ombiDefaultFilterAvailable',
      'ombiDefaultFilterUnavailable',
      'ombiDefaultFilterApproved',
      'ombiDefaultFilterUnapproved',
      'ombiDefaultFilterDenied',
      'ombiRefresh',
    ]);
  });
});

describe('the settings of the calendar', () => {
  it('have a group for SickRage with a connection test, and one for CouchPotato without', () => {
    expect(group('SickRage')?.test).toBe('sickrage');
    expect(group('SickRage')?.fields.map((one) => one.key)).toEqual([
      'homepageSickrageEnabled',
      'homepageSickrageAuth',
      'sickrageURL',
      'sickrageToken',
      'sickrageDisableCertCheck',
    ]);
    expect(group('CouchPotato')?.test).toBeUndefined();
    expect(group('CouchPotato')?.fields.map((one) => one.key)).toEqual([
      'homepageCouchpotatoEnabled',
      'homepageCouchpotatoAuth',
      'couchpotatoURL',
      'couchpotatoToken',
      'couchpotatoDisableCertCheck',
    ]);
    expect(field('sickrageToken')?.kind).toBe('secret');
    expect(field('couchpotatoToken')?.kind).toBe('secret');
  });

  it('have a group for the calendars in the iCal format', () => {
    expect(group('Calendars in the iCal format')?.fields.map((one) => one.key)).toEqual([
      'homepageCalendarEnabled',
      'homepageCalendarAuth',
      'calendariCal',
    ]);
  });
});

describe('the settings of the search items and of the monitors', () => {
  it('have a connection test where the server has one, and keep keys and passwords secret', () => {
    const tests: [string, string | undefined][] = [
      ['Jackett', 'jackett'],
      ['Prowlarr', 'prowlarr'],
      ['AdGuard Home', 'adguard'],
      ['UniFi', 'unifi'],
      ['Netdata', undefined],
      ['Monitorr', undefined],
      ['Speedtest Tracker', undefined],
      ['OctoPrint', undefined],
    ];
    for (const [title, test] of tests) {
      expect(group(title), title).toBeDefined();
      expect(group(title)?.test, title).toBe(test);
    }
    expect(group('UniFi')?.lookup?.test).toBe('unifi/site');
    for (const key of ['jackettToken', 'ProwlarrToken', 'octoprintToken', 'unifiPassword']) {
      expect(field(key)?.kind, key).toBe('secret');
    }
  });

  it('name the settings of Prowlarr and the places as the server names them', () => {
    expect(group('Prowlarr')?.fields.map((one) => one.key)).toEqual([
      'homepageProwlarrEnabled',
      'homepageProwlarrAuth',
      'homepageOrderProwlarr',
      'ProwlarrURL',
      'ProwlarrToken',
      'ProwlarrDisableCertCheck',
      'homepageProwlarrBackholeDownload',
    ]);
    expect(field('homepageOrderAdguard')).toBeDefined();
    expect(field('homepageOrderunifi')).toBeDefined();
    // The server has no setting for a login of AdGuard Home.
    expect(field('adGuardUsername')).toBeUndefined();
    expect(field('adGuardPassword')).toBeUndefined();
  });

  it('have a group for each of the seven charts of Netdata', () => {
    for (const number of [1, 2, 3, 4, 5, 6, 7]) {
      expect(group(`Netdata: chart ${number}`)?.fields.map((one) => one.key)).toEqual(
        ['Enabled', 'Title', 'Data', 'Chart', 'Colour', 'Size', 'lg', 'md', 'sm'].map(
          (name) => `netdata${number}${name}`,
        ),
      );
    }
    expect(field('netdataCustom')?.kind).toBe('textarea');
  });

  it('have each key once', () => {
    const keys = FIELDS.map((one) => one.key);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe('siteList', () => {
  it('names the sites of a controller with their descriptions', () => {
    expect(
      siteList({
        meta: { rc: 'ok' },
        data: [{ name: 'default', desc: 'Default' }, { name: 'x7k2m9qa' }, { desc: 'No name' }],
      }),
    ).toBe('The names of the sites are: Default: default, x7k2m9qa');
  });

  it('is nothing for an answer without sites', () => {
    expect(siteList(null)).toBeNull();
    expect(siteList({ content: [] })).toBeNull();
    expect(siteList({ data: [] })).toBeNull();
  });
});

describe('the tests of the settings pages', () => {
  it('have a button for Emby and for Jellyfin', () => {
    expect(group('Emby')?.test).toBe('emby');
    expect(group('Jellyfin')?.test).toBe('jellyfin');
  });

  it('test the webhook of the logs, which is a secret', () => {
    const slack = GENERAL.find((candidate) => candidate.title === 'Slack or Discord');
    expect(slack?.test).toBe('slack-logs');
    expect(slack?.fields.map((one) => [one.key, one.kind])).toEqual([
      ['sendLogsToSlack', 'switch'],
      ['slackLogWebhook', 'secret'],
      ['slackLogWebHookChannel', 'text'],
    ]);
  });

  it('test the schedules, the folder of the backups and the scheduler', () => {
    const fields = [...BACKUPS, ...HEALTH_CHECKS_PLUGIN].flatMap((one) => one.fields);
    const tested = fields.filter((one) => one.test).map((one) => [one.key, one.test]);
    expect(tested).toEqual([
      ['autoBackupCronSchedule', 'cron'],
      ['backupLocation', 'folder'],
      ['HEALTHCHECKS-cron-run-schedule', 'cron'],
    ]);
    expect(BACKUPS[0]?.scheduler).toBe(true);
    expect(HEALTH_CHECKS_PLUGIN[0]?.scheduler).toBe(true);
  });

  it('send a schedule as one text and a folder in the field folder', () => {
    expect(valueTestBody('cron', '@daily')).toBe('@daily');
    expect(valueTestBody('folder', '/backups')).toEqual({ folder: '/backups' });
  });
});

describe('the settings of YouTube', () => {
  it('have the API key as a secret, as the item Misc of PHP has it', () => {
    expect(group('YouTube')?.fields).toHaveLength(1);
    expect(field('youtubeAPI')).toMatchObject({ label: 'YouTube API key', kind: 'secret' });
  });
});
