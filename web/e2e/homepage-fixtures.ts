// The homepage items for the browser tests: for each kind of item, the settings that
// switch it on, and the answers of its routes. The answers follow the samples of the
// unit tests of the components (`src/pages/homepage/*.test.ts`). Every address is under
// the reserved top-level domain `.invalid`, and the browser never reaches the server
// for the data of an item: `withHomepageFixtures` answers each route itself, and every
// picture is a small picture of this file.

import type { Page, Request, Route } from '@playwright/test';

import { ADDRESS } from './server';

/** One route of an item and its answers: the `data` of the envelope of the API. */
export interface FixtureRoute {
  /**
   * The path under `api/v2/`, with `*` for one part of it, and the method in front when
   * it is not `GET`: `homepage/sabnzbd/queue`, `POST homepage/plex/metadata`.
   */
  pattern: string;
  normal: unknown;
  /** The answer when there is nothing to show. Without it, the normal answer. */
  empty?: unknown;
  /** The failure. Without it, a 500 with a message that names the item. */
  error?: { status: number; body: unknown };
}

export interface HomepageFixture {
  kind: string;
  settings: Record<string, unknown>;
  routes: FixtureRoute[];
}

export type Variant = 'normal' | 'empty' | 'error';

// ---------------------------------------------------------------- the downloaders

/** The queue of SABnzbd: two downloads, at 40 % and at 0 %, and one in the history. */
const SABNZBD_QUEUE = {
  content: {
    queueItems: {
      queue: {
        paused: false,
        speed: '1.2 M',
        slots: [
          {
            nzo_id: 'nzo_one',
            filename: 'The.Show.S02E03',
            status: 'Downloading',
            percentage: '40',
            timeleft: '0:10:00',
          },
          { nzo_id: 'nzo_two', filename: 'A.Film', status: 'Paused', percentage: '0' },
          {
            nzo_id: 'nzo_three',
            filename: 'A.Done.Film',
            status: 'Completed',
            percentage: '100',
            timeleft: '0:00:00',
          },
        ],
      },
    },
    historyItems: {
      history: {
        slots: [{ nzo_id: 'old', name: 'An.Old.Film', status: 'Completed', size: '4 GB' }],
      },
    },
  },
};

/** The downloads at 0 %, 50 % and 100 %, so the text of each fill is checked. */
const QBITTORRENT_QUEUE = {
  content: {
    queueItems: [
      { name: 'The.Show', state: 'downloading', progress: 0.5, eta: 3720 },
      { name: 'An.Old.Film', state: 'stalledUP', progress: 1, eta: 8640000 },
      { name: 'A.New.Kind', state: 'someNewState', progress: 0, eta: 8640000 },
    ],
    historyItems: false,
  },
};

const NZBGET_QUEUE = {
  content: {
    queueItems: {
      result: [
        { NZBName: 'The.Show', Status: 'DOWNLOADING', FileSizeMB: 1000, RemainingSizeMB: 500 },
        { NZBName: 'A.Film', Status: 'QUEUED', FileSizeMB: 0, RemainingSizeMB: 0 },
        { NZBName: 'A.Done.Film', Status: 'DOWNLOADING', FileSizeMB: 100, RemainingSizeMB: 0 },
      ],
    },
    historyItems: { result: [{ Name: 'An.Old.Film', Status: 'SUCCESS/ALL' }] },
  },
};

const TRANSMISSION_QUEUE = {
  content: {
    queueItems: [
      { name: 'The.Show', status: 4, percentDone: 0.5, eta: 3720 },
      { name: 'An.Old.Film', status: 6, percentDone: 1, eta: -1 },
      { name: 'A.Broken.One', status: 0, percentDone: 0, eta: -2, errorString: 'No data' },
      { name: 'A.New.Kind', status: 9, percentDone: 0, eta: -1 },
    ],
    historyItems: false,
  },
};

const DELUGE_QUEUE = {
  content: {
    queueItems: [
      {
        name: 'The.Show',
        state: 'Downloading',
        progress: 50,
        eta: 3720,
        tracker_status: 'Announce OK',
      },
      { name: 'An.Old.Film', state: 'Seeding', progress: 100, eta: 0, tracker_status: '' },
      { name: 'A.Waiting.One', state: 'Queued', progress: 0, eta: 0, tracker_status: '' },
    ],
    historyItems: false,
  },
};

const RTORRENT_QUEUE = {
  content: {
    queueItems: [
      // rTorrent sends its numbers as text.
      {
        name: 'The.Show',
        status: 'Downloading',
        size: '1000000',
        downloaded: '500000',
        label: 'tv',
      },
      { name: 'An.Old.Film', status: 'Seeding', size: '1000', downloaded: '1000', label: '' },
      { name: 'An.Empty.One', status: 'Stopped', size: '0', downloaded: '0', label: '' },
    ],
    historyItems: false,
  },
};

const UTORRENT_QUEUE = {
  content: {
    queueItems: [
      { Name: 'The.Show', Status: 'Downloading', Percent: '50.0%', Labels: 'tv' },
      { Name: 'An.Old.Film', Status: 'Seeding', Percent: '100.0%', Labels: '' },
      { Name: 'An.Odd.One', Status: '', Percent: '', Labels: '' },
    ],
  },
};

const JDOWNLOADER_QUEUE = {
  content: {
    queueItems: [
      {
        name: 'A.Package',
        speed: '1.2 MB/s',
        size: '4.00 GB',
        eta: '40m:00s',
        percentage: '50',
        enabled: true,
      },
      {
        name: 'A.Done.Package',
        speed: null,
        size: '1.00 GB',
        eta: null,
        percentage: '100',
        enabled: true,
      },
      {
        name: 'A.Waiting.Package',
        speed: null,
        size: '2.00 GB',
        eta: null,
        percentage: '0',
        enabled: true,
      },
      { name: 'A.Package.That.Is.Off', speed: null, size: '2.00 GB', eta: null, percentage: '0' },
    ],
    grabberItems: [{ name: 'A.New.Package' }],
    encryptedItems: [{ name: 'A.Locked.Package' }],
    offlineItems: [{ name: 'A.Lost.Package' }],
    $status: ['RUNNING', false, false],
  },
};

const ARR_QUEUE = {
  content: {
    queueItems: [
      {
        title: 'The.Show.S02E03.1080p',
        status: 'downloading',
        size: 1000,
        sizeleft: 500,
        timeleft: '01:51:22.3339332',
        series: { title: 'The Show' },
        episode: { seasonNumber: 2, episodeNumber: 3, title: 'The One' },
        movie: { title: 'The Film' },
      },
      { title: 'A.Release', status: 'queued', size: 0, sizeleft: 0 },
      { title: 'A.Done.Release', status: 'completed', size: 1000, sizeleft: 0 },
    ],
    historyItems: false,
  },
};

const NO_TORRENTS = { content: { queueItems: [], historyItems: false } };

// ---------------------------------------------------------------- the media servers

/** An item of Plex, Emby or Jellyfin, as the server builds it. */
function media(source: string, title: string, more: Record<string, unknown> = {}) {
  return {
    type: 'movie',
    title,
    secondaryTitle: '2019',
    summary: `The story of ${title}.`,
    uid: `${source}-${title}`,
    metadataKey: '815',
    imageURL: `api/v2/homepage/image?source=${source}&type=Primary&img=815&key=815-list`,
    nowPlayingImageURL: `api/v2/homepage/image?source=${source}&type=Thumb&img=815&key=815-np`,
    watched: 0,
    state: 'play',
    sessionType: 'Direct Playing',
    user: '',
    address: `https://${source}.invalid/web/item?id=815`,
    ...more,
  };
}

function streams(source: string) {
  return {
    content: [
      media(source, 'The Show', {
        type: 'tv',
        nowPlayingTitle: 'The Show - The One',
        nowPlayingBottom: 'S2 · E3',
        watched: 50,
        state: 'pause',
        user: 'robin',
        sessionType: 'Transcoding',
        bandwidth: 5100,
        bandwidthType: 'wan',
        userAddress: '203.0.113.7',
        nowPlayingOriginalImage: `api/v2/homepage/image?source=${source}&key=815-np$X`,
        userStream: {
          stream: 'transcode',
          videoDecision: 'Transcode',
          sourceVideoCodec: 'hevc',
          videoCodec: 'h264',
          videoResolution: '1080',
          audioDecision: 'Direct Play',
          sourceAudioCodec: 'aac',
          audioCodec: 'aac',
          product: 'Web',
          device: 'Firefox',
          platform: 'Chrome',
        },
      }),
      media(source, 'The Film', { watched: 0 }),
      media(source, 'The Long Film', { watched: 100, nowPlayingImageURL: '' }),
    ],
  };
}

function recent(source: string) {
  return {
    content: [
      media(source, 'The Film'),
      media(source, 'The Show', { type: 'tv' }),
      media(source, 'The Album', { type: 'music' }),
      // A poster without a picture shows the first letter of the title.
      media(source, 'No Picture', { imageURL: 'plugins/images/homepage/no-list.png' }),
    ],
  };
}

function details(source: string) {
  return {
    content: [
      media(source, 'The Film', {
        metadata: {
          summary: 'A long plot.',
          tagline: 'A made-up tagline',
          year: '2019',
          rating: '7.5',
          genres: ['Drama'],
          actors: [{ name: 'An Actor', role: 'The Hero' }],
        },
      }),
    ],
  };
}

/** The routes of a media server: the streams or the new items, and the details. */
function mediaRoutes(source: string, part: 'streams' | 'recent'): FixtureRoute[] {
  return [
    {
      pattern: `homepage/${source}/${part}`,
      normal: part === 'streams' ? streams(source) : recent(source),
      empty: { content: [] },
    },
    { pattern: `POST homepage/${source}/metadata`, normal: details(source) },
  ];
}

const PLEX = {
  homepagePlexEnabled: true,
  plexURL: 'https://plex.invalid',
  plexToken: 'fixture-token',
  plexID: 'fixture-machine',
};
const EMBY = { homepageEmbyEnabled: true, embyURL: 'https://emby.invalid', embyToken: 'fixture' };
const JELLYFIN = {
  homepageJellyfinEnabled: true,
  jellyfinURL: 'https://jellyfin.invalid',
  jellyfinToken: 'fixture-token',
};

// ---------------------------------------------------------------- the requests

function request(id: number, title: string, more: Record<string, unknown> = {}) {
  return {
    id: id + 800,
    request_id: id,
    type: 'movie',
    title,
    overview: `The story of ${title}.`,
    poster: `https://image.invalid/t/p/w300/${id}.jpg`,
    background: '',
    approved: false,
    available: false,
    denied: false,
    deniedReason: null,
    user: 'pat',
    userAlias: 'Pat',
    request_date: '2020-01-05T10:00:00.000Z',
    release_date: '2019-05-01',
    ...more,
  };
}

const REQUESTS = {
  count: { movie: 3, tv: 1 },
  content: [
    request(31, 'The Film', { approved: true, available: true }),
    request(32, 'Waiting Film'),
    request(33, 'Denied Film', { denied: true, poster: 'plugins/images/homepage/no-list.png' }),
    request(34, 'The Show', { type: 'tv', approved: true }),
  ],
};

const FOUND = {
  page: 1,
  totalPages: 2,
  results: [
    {
      id: 815,
      type: 'movie',
      title: 'The Film',
      year: '2019',
      poster: 'https://image.invalid/t/p/w300/815.jpg',
      overview: 'A made-up film.',
      status: null,
    },
    {
      id: 816,
      type: 'tv',
      title: 'The Show',
      year: '2020',
      poster: 'https://image.invalid/t/p/w300/816.jpg',
      overview: '',
      status: 'available',
    },
    {
      id: 817,
      type: 'movie',
      title: 'The Sequel',
      year: '2021',
      poster: 'https://image.invalid/t/p/w300/817.jpg',
      overview: 'More of the same.',
      status: 'pending',
    },
  ],
};

function requestRoutes(service: string): FixtureRoute[] {
  return [
    {
      pattern: `homepage/${service}/requests`,
      normal: REQUESTS,
      empty: { count: { movie: 0, tv: 0 }, content: [] },
    },
    {
      pattern: `homepage/${service}/search/*`,
      normal: FOUND,
      empty: { page: 1, totalPages: 1, results: [] },
    },
    { pattern: `homepage/${service}/search/*/*`, normal: { ...FOUND, page: 2, results: [] } },
  ];
}

// ---------------------------------------------------------------- the indexers

const JACKETT = {
  content: {
    Results: [
      {
        PublishDate: '2026-10-20T08:30:00+00:00',
        Tracker: 'Tracker two',
        Title: 'A Film 2024 720p',
        Details: null,
        Size: 1073741824,
        Files: null,
        Grabs: null,
        Seeders: 3,
        Peers: 1,
        MagnetUri: 'magnet:?xt=urn:btih:00',
        BlackholeLink: null,
        Guid: 'magnet:?xt=urn:btih:00',
        Link: null,
      },
      {
        PublishDate: '2026-11-01T10:00:00+00:00',
        Tracker: 'Tracker one',
        Title: 'A Film 2024 1080p',
        Details: 'https://tracker.invalid/details/1',
        Size: 4294967296,
        Files: 3,
        Grabs: 120,
        Seeders: 45,
        Peers: 7,
        MagnetUri: null,
        BlackholeLink: 'https://jackett.invalid/bh/tracker/?path=abc&file=A+Film',
        Guid: 'https://tracker.invalid/details/1',
        Link: 'https://jackett.invalid/dl/tracker/?path=abc&file=A+Film',
      },
    ],
  },
};

const PROWLARR = {
  content: [
    {
      guid: 'https://tracker.invalid/details/1',
      indexerId: 5,
      indexer: 'Tracker one',
      title: 'A Film 2024 1080p',
      size: 4294967296,
      publishDate: '2026-11-01T10:00:00Z',
      downloadUrl: 'https://prowlarr.invalid/5/download?link=abc',
      infoUrl: 'https://tracker.invalid/details/1',
      grabs: 120,
      seeders: 45,
      leechers: 7,
    },
    {
      guid: 'https://tracker.invalid/details/2',
      indexerId: 5,
      indexer: 'Tracker one',
      title: 'A Film 2024 2160p',
      size: 1536,
      publishDate: '2026-11-02T10:00:00Z',
      infoUrl: null,
      grabs: 1,
      seeders: 90,
      leechers: 2,
    },
  ],
};

// ---------------------------------------------------------------- the calendar

/** A day of the month of today, as the calendar writes it. */
function dayOfThisMonth(day: number): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  return `${now.getFullYear()}-${month}-${String(day).padStart(2, '0')}`;
}

function today(): string {
  return dayOfThisMonth(new Date().getDate());
}

function event(id: string, start: string, kind: string, state: string) {
  return {
    id,
    title: id,
    start,
    imagetypeFilter: kind,
    downloadFilter: state,
    releaseType: kind === 'film' ? ['digital'] : undefined,
    details: {
      topTitle: id,
      bottomTitle: `About ${id}`,
      overview: `The story of ${id}.`,
      runtime: 45,
      image: 'https://image.invalid/calendar/815.jpg',
      ratings: 8.1,
      genres: ['Comedy'],
      href: 'https://sonarr.invalid/series/the-show',
      showLink: true,
    },
  };
}

/** An event in each state, on today (for the week and the day) and on other days. */
function calendarEvents() {
  return {
    ical: [
      { title: 'Dinner', start: today(), end: today(), bgColor: 'bg-info' },
      { title: 'Lunch', start: dayOfThisMonth(1), end: dayOfThisMonth(1), bgColor: 'bg-success' },
    ],
    events: [
      event('Downloaded Show', today(), 'tv', 'text-success'),
      event('Unreleased Film', today(), 'film', 'text-info'),
      event('Missing Film', today(), 'film', 'text-danger'),
      event('Premiere Show', `${today()}T20:00:00Z`, 'tv', 'text-primary animated flash'),
      event('Unmonitored Album', today(), 'music', 'text-dark'),
      event('Another Show', dayOfThisMonth(12), 'tv', 'text-success'),
      event('Another Missing', dayOfThisMonth(20), 'tv', 'text-danger'),
    ],
  };
}

// ---------------------------------------------------------------- every item

export const HOMEPAGE_FIXTURES: HomepageFixture[] = [
  {
    kind: 'customhtml',
    // The frame is a document of the server itself, which sends nothing anywhere.
    settings: {
      homepageCustomHTML01Enabled: true,
      customHTML01: '<p>Hello from custom HTML</p>',
    },
    routes: [],
  },
  {
    kind: 'bookmarks',
    settings: { homepageBookmarksEnabled: true, 'BOOKMARK-enabled': true },
    routes: [
      {
        pattern: 'plugins/bookmark/page',
        normal: {
          categories: [
            {
              category: 'Work',
              tabs: [
                {
                  name: 'Wiki',
                  url: 'https://wiki.invalid',
                  image: 'fontawesome::bookmark',
                  text_color: '#ffffff',
                  border_color: '#a7a7a7',
                  dark_color: '#5a5a5a',
                  light_color: '#8d8d8d',
                },
              ],
            },
          ],
        },
        empty: { categories: [] },
      },
    ],
  },
  {
    kind: 'healthchecks',
    settings: {
      homepageHealthChecksEnabled: true,
      healthChecksURL: 'https://checks.invalid/api/v1/checks',
      healthChecksToken: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/healthchecks',
        normal: {
          content: {
            checks: ['up', 'down', 'grace', 'paused', 'new'].map((status) => ({
              name: `Check ${status}`,
              status,
              tags: 'prod https://runbook.invalid',
              desc: `Notes of the check that is ${status}`,
              last_ping: '2020-01-01T10:00:00+00:00',
              next_ping: '2020-01-02T10:00:00+00:00',
            })),
          },
          options: { desc: true, tags: true },
        },
        empty: { content: { checks: [] }, options: {} },
      },
    ],
  },
  {
    kind: 'uptimekuma',
    settings: {
      homepageUptimeKumaEnabled: true,
      uptimeKumaURL: 'https://kuma.invalid',
      uptimeKumaToken: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/kuma/data',
        normal: {
          data: [
            { name: 'Router', type: 'ping', url: 'null', status: true, latency: 3 },
            {
              name: 'Blog',
              type: 'http',
              url: 'https://blog.invalid',
              status: false,
              latency: null,
            },
          ],
          options: { title: 'Monitors', titleToggle: true, compact: true, showLatency: true },
        },
        empty: {
          data: [],
          options: { title: 'Monitors', titleToggle: true, compact: true, showLatency: true },
        },
      },
    ],
  },
  {
    kind: 'calendar',
    settings: {
      homepageSonarrEnabled: true,
      sonarrURL: 'https://sonarr.invalid',
      sonarrToken: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/calendar',
        normal: calendarEvents(),
        empty: { ical: false, events: [] },
      },
    ],
  },
  {
    kind: 'sonarrqueue',
    settings: {
      homepageSonarrEnabled: true,
      homepageSonarrQueueEnabled: true,
      sonarrURL: 'https://sonarr.invalid',
      sonarrToken: 'fixture-token',
    },
    routes: [{ pattern: 'homepage/sonarr/queue', normal: ARR_QUEUE, empty: NO_TORRENTS }],
  },
  {
    kind: 'radarrqueue',
    settings: {
      homepageRadarrEnabled: true,
      homepageRadarrQueueEnabled: true,
      radarrURL: 'https://radarr.invalid',
      radarrToken: 'fixture-token',
    },
    routes: [{ pattern: 'homepage/radarr/queue', normal: ARR_QUEUE, empty: NO_TORRENTS }],
  },
  {
    kind: 'sabnzbd',
    settings: {
      homepageSabnzbdEnabled: true,
      sabnzbdURL: 'https://sabnzbd.invalid',
      sabnzbdToken: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/sabnzbd/queue',
        normal: SABNZBD_QUEUE,
        empty: {
          content: {
            queueItems: { queue: { paused: true, slots: [] } },
            historyItems: { history: { slots: [] } },
          },
        },
      },
    ],
  },
  {
    kind: 'qbittorrent',
    settings: { homepageqBittorrentEnabled: true, qBittorrentURL: 'https://qbittorrent.invalid' },
    routes: [
      { pattern: 'homepage/qbittorrent/queue', normal: QBITTORRENT_QUEUE, empty: NO_TORRENTS },
    ],
  },
  {
    kind: 'nzbget',
    settings: { homepageNzbgetEnabled: true, nzbgetURL: 'https://nzbget.invalid' },
    routes: [
      {
        pattern: 'homepage/nzbget/queue',
        normal: NZBGET_QUEUE,
        empty: { content: { queueItems: { result: [] }, historyItems: { result: [] } } },
      },
    ],
  },
  {
    kind: 'transmission',
    settings: {
      homepageTransmissionEnabled: true,
      transmissionURL: 'https://transmission.invalid',
    },
    routes: [
      { pattern: 'homepage/transmission/queue', normal: TRANSMISSION_QUEUE, empty: NO_TORRENTS },
    ],
  },
  {
    kind: 'deluge',
    settings: { homepageDelugeEnabled: true, delugeURL: 'https://deluge.invalid' },
    routes: [{ pattern: 'homepage/deluge/queue', normal: DELUGE_QUEUE, empty: NO_TORRENTS }],
  },
  {
    kind: 'rtorrent',
    settings: { homepagerTorrentEnabled: true, rTorrentURL: 'https://rtorrent.invalid' },
    routes: [{ pattern: 'homepage/rtorrent/queue', normal: RTORRENT_QUEUE, empty: NO_TORRENTS }],
  },
  {
    kind: 'utorrent',
    settings: { homepageuTorrentEnabled: true, uTorrentURL: 'https://utorrent.invalid' },
    routes: [
      {
        pattern: 'homepage/utorrent/queue',
        normal: UTORRENT_QUEUE,
        empty: { content: { queueItems: [] } },
      },
    ],
  },
  {
    kind: 'jdownloader',
    settings: { homepageJdownloaderEnabled: true, jdownloaderURL: 'https://jdownloader.invalid' },
    routes: [
      {
        pattern: 'homepage/jdownloader/queue',
        normal: JDOWNLOADER_QUEUE,
        empty: {
          content: {
            queueItems: [],
            grabberItems: [],
            encryptedItems: [],
            offlineItems: [],
            $status: [null, null, null],
          },
        },
      },
    ],
  },
  {
    kind: 'pihole',
    settings: { homepagePiholeEnabled: true, piholeURL: 'https://pihole.invalid' },
    routes: [
      {
        pattern: 'homepage/pihole/stats',
        normal: {
          data: {
            'https://pihole.invalid': {
              sum_queries: 12000,
              sum_blocked: 3000,
              percent_blocked: 25,
              domains_being_blocked: ['ads.invalid', 'tracker.invalid'],
            },
            'https://pihole-two.invalid': {
              sum_queries: 8000,
              sum_blocked: 1000,
              percent_blocked: 12.5,
              domains_being_blocked: ['ads.invalid', 'metrics.invalid'],
            },
          },
          options: { combine: false, title: true },
        },
        empty: {
          data: { 'https://pihole.invalid': { domains_being_blocked: [] } },
          options: { combine: false, title: true },
        },
      },
    ],
  },
  {
    kind: 'weather',
    settings: {
      homepageWeatherAndAirEnabled: true,
      homepageWeatherAndAirLatitude: '51.5',
      homepageWeatherAndAirLongitude: '-0.12',
      homepageWeatherAndAirWeatherHeaderToggle: true,
      homepageWeatherAndAirWeatherHeader: 'At home',
    },
    routes: [
      {
        pattern: 'homepage/weather/data',
        normal: {
          content: {
            weather: {
              data: [
                ['2020-01-05T12:00:00Z', 'Partly cloudy', 7.5],
                ['2020-01-05T13:00:00Z', 'Cloudy', 8],
                ['2020-01-05T14:00:00Z', 'Rain', 6],
              ].map(([time, text, degrees]) => ({
                datetime: time,
                weather_text: text,
                temperature: { value: degrees, units: 'C' },
                feels_like_temperature: { value: Number(degrees) - 1, units: 'C' },
                relative_humidity: 61,
                wind: { speed: { value: 12.4, units: 'km/h' } },
              })),
            },
            air: {
              data: {
                indexes: { baqi: { aqi_display: '67', category: 'Good air quality' } },
                health_recommendations: { general_population: 'You have no limitations.' },
              },
            },
            pollen: {
              data: [
                {
                  types: {
                    grass: {
                      display_name: 'Grass',
                      data_available: true,
                      index: { category: 'Very Low' },
                    },
                    weed: {
                      display_name: 'Weed',
                      data_available: false,
                      index: { category: null },
                    },
                  },
                },
              ],
            },
          },
        },
        empty: { content: { weather: false, air: false, pollen: false } },
      },
    ],
  },
  {
    kind: 'tautulli',
    settings: {
      homepageTautulliEnabled: true,
      tautulliURL: 'https://tautulli.invalid',
      tautulliApikey: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/tautulli/data',
        normal: {
          options: {
            libraries: true,
            topMovies: true,
            topTV: true,
            topUsers: true,
            topPlatforms: true,
            popularMovies: true,
            popularTV: true,
            title: true,
            friendlyName: true,
          },
          homestats: {
            data: [
              {
                stat_id: 'top_movies',
                rows: [
                  { title: 'The Film', total_plays: 12, thumb: 'data/cache/tautulli-815-list.jpg' },
                  { title: 'Another Film', total_plays: 9, thumb: '/library/metadata/816/thumb/1' },
                  { title: 'A Third Film', total_plays: 3 },
                ],
              },
              { stat_id: 'popular_tv', rows: [{ title: 'The Show', users_watched: 4 }] },
              {
                stat_id: 'top_users',
                rows: [{ user: 'robin', friendly_name: 'Robin', total_plays: 30 }],
              },
              { stat_id: 'top_platforms', rows: [{ platform: 'Chrome', total_plays: 20 }] },
            ],
          },
          libstats: {
            data: [
              { section_name: 'Films', section_type: 'movie', count: 120, plays: 300 },
              {
                section_name: 'Series',
                section_type: 'show',
                count: 10,
                parent_count: 40,
                child_count: 500,
                plays: 900,
              },
              { section_name: 'Songs', section_type: 'artist', count: 5, plays: 9 },
            ],
          },
        },
        empty: {
          options: { libraries: true, title: true },
          homestats: { data: [] },
          libstats: { data: [] },
        },
      },
    ],
  },
  {
    kind: 'plexstreams',
    settings: { ...PLEX, homepagePlexStreams: true },
    routes: mediaRoutes('plex', 'streams'),
  },
  {
    kind: 'plexrecent',
    settings: { ...PLEX, homepagePlexRecent: true },
    routes: mediaRoutes('plex', 'recent'),
  },
  {
    kind: 'plexplaylists',
    settings: { ...PLEX, homepagePlexPlaylist: true },
    routes: [
      {
        pattern: 'homepage/plex/playlists',
        normal: {
          content: {
            Filmnight: {
              title: 'Film night!',
              0: media('plex', 'The Film'),
              1: media('plex', 'Another Film'),
            },
            Kids: { title: 'Kids', 0: media('plex', 'A Film For Children') },
          },
        },
        empty: { content: {} },
      },
    ],
  },
  {
    kind: 'embystreams',
    settings: { ...EMBY, homepageEmbyStreams: true },
    routes: mediaRoutes('emby', 'streams'),
  },
  {
    kind: 'embyrecent',
    settings: { ...EMBY, homepageEmbyRecent: true },
    routes: mediaRoutes('emby', 'recent'),
  },
  {
    kind: 'jellyfinstreams',
    settings: { ...JELLYFIN, homepageJellyfinStreams: true },
    routes: mediaRoutes('jellyfin', 'streams'),
  },
  {
    kind: 'jellyfinrecent',
    settings: { ...JELLYFIN, homepageJellyfinRecent: true },
    routes: mediaRoutes('jellyfin', 'recent'),
  },
  {
    kind: 'overseerr',
    // The search of the requests shows with the session of the service (a cookie that
    // the test sets) and its single sign-on.
    settings: {
      homepageOverseerrEnabled: true,
      overseerrURL: 'https://overseerr.invalid',
      overseerrToken: 'fixture-token',
      ssoOverseerr: true,
    },
    routes: requestRoutes('overseerr'),
  },
  {
    kind: 'ombi',
    settings: {
      homepageOmbiEnabled: true,
      ombiURL: 'https://ombi.invalid',
      ombiToken: 'fixture-token',
      ssoOmbi: true,
    },
    routes: requestRoutes('ombi'),
  },
  {
    kind: 'jackett',
    settings: {
      homepageJackettEnabled: true,
      jackettURL: 'https://jackett.invalid',
      jackettToken: 'fixture-token',
    },
    routes: [
      { pattern: 'homepage/jackett/*', normal: JACKETT, empty: { content: { Results: [] } } },
    ],
  },
  {
    kind: 'prowlarr',
    settings: {
      homepageProwlarrEnabled: true,
      prowlarrURL: 'https://prowlarr.invalid',
      prowlarrToken: 'fixture-token',
    },
    routes: [{ pattern: 'homepage/prowlarr/*', normal: PROWLARR, empty: { content: [] } }],
  },
  {
    kind: 'adguard',
    settings: { homepageAdGuardEnabled: true, adguardURL: 'https://adguard.invalid' },
    routes: [
      {
        pattern: 'homepage/adguard/stats',
        normal: {
          data: {
            '192.0.2.5': {
              num_dns_queries: 12000,
              num_blocked_filtering: 3000,
              avg_processing_time: 0.01234,
            },
            '192.0.2.6': {
              num_dns_queries: 8000,
              num_blocked_filtering: 1000,
              avg_processing_time: 0.002,
            },
          },
          filters: {
            '192.0.2.5': {
              filters: [{ rules_count: 100000 }, { rules_count: 50000 }],
              user_rules: ['||ads.invalid^', '@@||good.invalid^'],
            },
            '192.0.2.6': { filters: null, user_rules: [] },
          },
          options: {
            combine: false,
            title: true,
            queries: true,
            blocked_count: true,
            blocked_percent: true,
            processing_time: true,
            domain_count: true,
          },
        },
        empty: { data: null, filters: null, options: { combine: false, title: true } },
      },
    ],
  },
  {
    kind: 'netdata',
    settings: { homepageNetdataEnabled: true, netdataURL: 'https://netdata.invalid' },
    routes: [
      {
        pattern: 'homepage/netdata/data',
        normal: {
          url: '',
          data: [
            {
              title: 'CPU',
              value: 12.54,
              percent: 12.54,
              units: '%',
              max: 100,
              chart: 'easypiechart',
              colour: '46e302',
              size: 'lg',
              lg: true,
              md: true,
              sm: true,
            },
            {
              title: 'Disk write',
              value: 3,
              percent: 50,
              units: 'MiB/s',
              max: 6000,
              chart: 'gauge',
              colour: '',
              size: '',
              lg: true,
              md: true,
              sm: true,
            },
            {
              error: 'custom definition not found',
              title: 'Custom',
              chart: 'gauge',
              lg: true,
              md: true,
              sm: true,
            },
          ],
        },
        empty: { url: '', data: [] },
      },
    ],
  },
  {
    kind: 'monitorr',
    settings: { homepageMonitorrEnabled: true, monitorrURL: 'https://monitorr.invalid' },
    routes: [
      {
        pattern: 'homepage/monitorr/data',
        normal: {
          services: {
            'Backup & Sync': {
              status: false,
              sort: 2,
              image: 'plugins/images/homepage/no-list.png',
            },
            Plex: {
              status: true,
              sort: 0,
              image: 'https://monitorr.invalid/img/plex.png',
              link: 'https://plex.invalid/web',
            },
            Router: {
              status: 'unresponsive',
              sort: 1,
              image: 'https://monitorr.invalid/img/router.png',
            },
          },
          options: { title: 'Monitorr', titleToggle: true, compact: false },
        },
        empty: { services: {}, options: { title: 'Monitorr', titleToggle: true, compact: false } },
      },
    ],
  },
  {
    kind: 'speedtest',
    settings: { homepageSpeedtestEnabled: true, speedtestURL: 'https://speedtest.invalid' },
    routes: [
      {
        pattern: 'homepage/speedtest/data',
        normal: {
          data: {
            current: { ping: '12.345', download: 250.12, upload: 40.26 },
            average: { ping: 13.5, download: 240, upload: 39 },
            maximum: { ping: 20, download: 260.5, upload: 41 },
            minimum: { ping: 9.5, download: 200.25, upload: 35 },
          },
          options: { title: 'Line', titleToggle: true },
        },
        empty: { data: { current: null }, options: { title: 'Line', titleToggle: true } },
      },
    ],
  },
  {
    kind: 'netronome',
    settings: { homepageNetronomeEnabled: true, netronomeURL: 'https://netronome.invalid' },
    routes: [
      {
        pattern: 'homepage/netronome/data',
        normal: {
          data: {
            current: {
              ping: 12.345,
              download: 250.12,
              upload: 40.26,
              jitter: 1.25,
              server: 'A server',
              type: 'speedtest',
              time: '2026-11-03T10:00:00Z',
            },
            average: { ping: 13.5, download: 240, upload: 39 },
            maximum: { ping: 20, download: 260.5, upload: 41 },
            minimum: { ping: 9.5, download: 200.25, upload: 35 },
            tests: 3,
          },
          options: { title: 'Line speed', titleToggle: true, timeRange: 'week' },
        },
        empty: {
          data: { current: null },
          options: { title: 'Line speed', titleToggle: true, timeRange: 'week' },
        },
      },
    ],
  },
  {
    kind: 'octoprint',
    settings: {
      homepageOctoprintEnabled: true,
      octoprintURL: 'https://printer.invalid',
      octoprintToken: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/octoprint/data',
        normal: {
          data: {
            url: 'https://printer.invalid/',
            job: {
              state: 'Printing',
              job: {
                file: { display: 'Bracket.gcode', name: 'bracket.gcode' },
                estimatedPrintTime: 5400,
              },
              progress: { completion: 41.6, printTimeLeft: 3200 },
            },
            settings: { webcam: { webcamEnabled: true, streamUrl: '/webcam/?action=stream' } },
          },
          options: { title: 'Octoprint', titleToggle: true },
        },
        empty: {
          data: {
            url: 'https://printer.invalid/',
            job: {
              state: 'Operational',
              job: { file: { display: null } },
              progress: { completion: null },
            },
            settings: { webcam: { webcamEnabled: false } },
          },
          options: { title: 'Octoprint', titleToggle: true },
        },
      },
    ],
  },
  {
    kind: 'unifi',
    settings: {
      homepageUnifiEnabled: true,
      unifiURL: 'https://unifi.invalid',
      unifiUsername: 'fixture',
      unifiPassword: 'fixture-password',
    },
    routes: [
      {
        pattern: 'homepage/unifi/data',
        normal: {
          content: {
            unifi: {
              meta: { rc: 'ok' },
              data: [
                {
                  subsystem: 'wlan',
                  status: 'ok',
                  num_user: 14,
                  'tx_bytes-r': 120000,
                  'rx_bytes-r': 45000,
                },
                { subsystem: 'wan', status: 'ok', wan_ip: '203.0.113.7', 'tx_bytes-r': 300000 },
                { subsystem: 'www', status: 'ok', drops: 2, latency: 11, uptime: 86400 },
              ],
            },
          },
        },
        empty: { content: { unifi: [] } },
      },
    ],
  },
  {
    kind: 'prompage',
    settings: { homepagePromPageEnabled: true, promPageURL: 'https://prompage.invalid' },
    routes: [
      {
        pattern: 'homepage/prompage/data',
        normal: {
          data: [
            { name: 'Blog', status: true, uptime: 99.954 },
            { name: 'Mail', status: false, uptime: null },
          ],
          options: { title: 'Our services', titleToggle: true, compact: true, showUptime: true },
        },
        empty: {
          data: null,
          options: { title: 'Our services', titleToggle: true, compact: true, showUptime: true },
        },
      },
    ],
  },
  {
    kind: 'donate',
    settings: {
      homepageDonateEnabled: true,
      homepageDonatePublicToken: 'pk_test_fixture',
      homepageDonateSecretToken: 'sk_test_fixture',
      homepageDonateProductID: 'prod_fixture',
      homepageDonateShowUserHistory: true,
    },
    routes: [
      {
        pattern: 'homepage/donate',
        normal: [{ date: 'last week', email: 'robin@donate.invalid', amount: 12.5 }],
        empty: [],
      },
    ],
  },
  {
    kind: 'embylivetv',
    settings: { ...EMBY, homepageEmbyLiveTVTrackerEnabled: true },
    routes: [
      {
        pattern: 'homepage/embyLiveTVTracker/stats',
        normal: { activeTimers: 2, seriesTimers: 1, todaysRecordings: 0, totalRecordings: 31 },
      },
      {
        pattern: 'homepage/embyLiveTVTracker/activity',
        normal: {
          scheduledRecordings: [
            {
              date: 'tonight',
              name: 'Guests',
              seriesName: 'The Late Show',
              episodeTitle: 'Guests',
              channelName: 'Channel 12',
              userName: 'Zed',
              duration: '1h 30m',
              status: 'Scheduled',
            },
          ],
          completedRecordings: [
            { date: 'yesterday', name: 'The Film', seriesName: '', duration: '1h 30m' },
          ],
        },
        empty: { scheduledRecordings: [], completedRecordings: [] },
      },
    ],
  },
  {
    kind: 'jellystat',
    settings: {
      homepageJellyStatEnabled: true,
      jellyStatURL: 'https://jellystat.invalid',
      jellyStatApikey: 'fixture-token',
    },
    routes: [
      {
        pattern: 'homepage/jellystat',
        normal: {
          period: '30 days',
          libraries: [
            { name: 'Films', type: 'Movies', item_count: 120, season_count: 0, episode_count: 0 },
            { name: 'Series', type: 'Shows', item_count: 10, season_count: 40, episode_count: 500 },
          ],
          library_totals: {
            total_libraries: 2,
            total_items: 130,
            total_seasons: 40,
            total_episodes: 500,
          },
          users: [
            { name: 'robin', play_count: 3, last_activity: '2020-01-05T10:00:00Z' },
            { name: 'pat', play_count: 1, last_activity: null },
          ],
          most_watched_movies: [{ id: '815', title: 'The Film', play_count: 2, year: '2019' }],
          most_watched_shows: [{ id: '816', title: 'The Show', play_count: 1, year: null }],
          most_listened_music: [],
        },
        empty: { period: '30 days', libraries: [], library_totals: [], users: [] },
      },
      {
        pattern: 'POST homepage/jellystat/metadata',
        normal: {
          content: [
            {
              title: 'The Film',
              metadata: { summary: 'A film.', year: '2019' },
              address: 'https://jellyfin.invalid/item?id=815',
            },
          ],
        },
      },
    ],
  },
];

/** The downloaders that can share one item with tabs, and the switch of each one. */
export const COMBINE_SWITCHES: Record<string, string> = {
  sabnzbd: 'sabnzbdCombine',
  nzbget: 'nzbgetCombine',
  transmission: 'transmissionCombine',
  qbittorrent: 'qBittorrentCombine',
  deluge: 'delugeCombine',
  rtorrent: 'rTorrentCombine',
  utorrent: 'uTorrentCombine',
  jdownloader: 'jdownloaderCombine',
  sonarrqueue: 'homepageSonarrQueueCombine',
  radarrqueue: 'homepageRadarrQueueCombine',
};

/** The settings that switch every item on, with `.invalid` addresses. */
export function allItemSettings(): Record<string, unknown> {
  return Object.assign({}, ...HOMEPAGE_FIXTURES.map((fixture) => fixture.settings));
}

// ---------------------------------------------------------------- the routes

/** A picture of one transparent pixel, for every picture of the items. */
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
  'base64',
);

/** The routes of the server that serve the homepage from its own data: no item data. */
const OWN_ROUTES = [/^homepage\/items$/, /^homepage\/html\/\d+$/];

/** The requests that no fixture answered, by page. */
const unrouted = new WeakMap<Page, string[]>();

/** The requests for item data that no fixture answered, and those to another host. */
export function unroutedRequests(page: Page): string[] {
  return unrouted.get(page) ?? [];
}

/** How many times each route was answered, by page. */
const answered = new WeakMap<Page, Map<string, number>>();

/** How many times the route of `pattern` was answered on the page. */
export function timesAnswered(page: Page, pattern: string): number {
  return answered.get(page)?.get(pattern) ?? 0;
}

function envelope(data: unknown) {
  return { response: { result: 'success', message: null, data } };
}

function failure(message: string) {
  return { response: { result: 'error', message, data: null } };
}

/** The method and the regular expression of a pattern. */
function parse(pattern: string): { method: string; path: RegExp } {
  const [method, path] = pattern.includes(' ') ? pattern.split(' ') : ['GET', pattern];
  const escaped = (path ?? '')
    .split('*')
    .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, '\\$&'))
    .join('[^/]+');
  return { method: method ?? 'GET', path: new RegExp(`^${escaped}$`) };
}

/** The path of a request under `api/v2/`, or nothing for another address. */
function apiPath(request: Request, origin: string): string | null {
  const address = new URL(request.url());
  if (address.origin !== origin) {
    return null;
  }
  const at = address.pathname.indexOf('/api/v2/');
  return at === -1 ? null : decodeURIComponent(address.pathname.slice(at + '/api/v2/'.length));
}

/**
 * Answers every route of every item on the page with the answers of `variant`, every
 * picture with one pixel, and refuses every other request for item data and every
 * request to another host (they are listed by `unroutedRequests`).
 */
export async function withHomepageFixtures(page: Page, variant: Variant): Promise<void> {
  await page.unrouteAll({ behavior: 'wait' });
  const missed: string[] = [];
  unrouted.set(page, missed);
  const counts = new Map<string, number>();
  answered.set(page, counts);
  const origin = `http://${ADDRESS}`;
  const routes = HOMEPAGE_FIXTURES.flatMap((fixture) =>
    fixture.routes.map((route) => ({ ...route, kind: fixture.kind, ...parse(route.pattern) })),
  );

  await page.route('**/*', async (route: Route) => {
    const request = route.request();
    const address = new URL(request.url());
    if (
      request.resourceType() === 'image' &&
      (address.origin !== origin || isItemPicture(address))
    ) {
      await route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL });
      return;
    }
    if (address.origin !== origin) {
      missed.push(`${request.method()} ${request.url()}`);
      await route.abort('blockedbyclient');
      return;
    }
    const path = apiPath(request, origin);
    const isItemData =
      path !== null && (path.startsWith('homepage/') || path.startsWith('plugins/bookmark/'));
    if (path === null || !isItemData || OWN_ROUTES.some((own) => own.test(path))) {
      await route.fallback();
      return;
    }
    // The most exact pattern first: `search/*/*` before `search/*`, `jackett/*` last.
    const match = routes
      .filter((one) => one.method === request.method() && one.path.test(path))
      .sort((a, b) => b.pattern.length - a.pattern.length)[0];
    if (!match) {
      missed.push(`${request.method()} ${path}`);
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify(failure(`No fixture answers ${path}`)),
      });
      return;
    }
    counts.set(match.pattern, (counts.get(match.pattern) ?? 0) + 1);
    if (variant === 'error') {
      const error = match.error ?? {
        status: 500,
        body: failure(`The ${match.kind} service cannot be reached`),
      };
      await route.fulfill({
        status: error.status,
        contentType: 'application/json',
        body: JSON.stringify(error.body),
      });
      return;
    }
    const data = variant === 'empty' && 'empty' in match ? match.empty : match.normal;
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(envelope(data)),
    });
  });
}

/** A picture of an item on the server: from the cache of the server, or through it. */
function isItemPicture(address: URL): boolean {
  return /\/(api\/v2\/homepage\/image|data\/cache\/|library\/)/.test(address.pathname);
}
