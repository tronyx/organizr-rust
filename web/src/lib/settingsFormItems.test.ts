import { describe, expect, it } from 'vitest';

import { type Field, HOMEPAGE } from './settingsForm';

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
