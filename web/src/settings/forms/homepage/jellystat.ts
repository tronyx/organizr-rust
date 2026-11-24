// The homepage item JellyStat: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const JELLYSTAT: HomepageItemForm = {
  id: 'jellystat',
  name: 'JellyStat',
  image: 'plugins/images/homepage/jellystat.png',
  category: 'Media Server',
  enabledKey: 'homepageJellyStatEnabled',
  page: {
    id: 'homepage-jellystat',
    title: 'JellyStat',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageJellyStatEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageJellyStatAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
        ],
      },
      {
        id: 'connection',
        title: 'Connection',
        entries: [
          {
            key: 'jellyStatURL',
            kind: 'text',
            label: 'JellyStat URL',
            help: 'URL to your JellyStat instance',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'jellyStatInternalURL',
            kind: 'text',
            label: 'Internal JellyStat URL (optional)',
            help: 'Internal URL for server-side API calls (e.g., http://192.168.80.77:3000). If not set, uses main URL.',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'jellyStatApikey',
            kind: 'secret',
            label: 'JellyStat API Key',
            help: 'API key for JellyStat (required for native mode)',
          },
          { key: 'jellyStatDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'jellyStatUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/jellystat', method: 'POST' },
          },
        ],
      },
      {
        id: 'display-mode',
        title: 'Display mode',
        entries: [
          {
            key: 'homepageJellyStatDisplayMode',
            kind: 'select',
            label: 'Display Mode',
            choices: [
              { label: 'Native Statistics View', value: 'native' },
              { label: 'Embedded JellyStat Interface', value: 'iframe' },
            ],
          },
        ],
      },
      {
        id: 'native-mode',
        title: 'Native mode',
        entries: [
          {
            key: 'homepageJellyStatRefresh',
            kind: 'number',
            label: 'Auto-refresh Interval (minutes)',
            min: 1,
            max: 60,
          },
          {
            key: 'homepageJellyStatDays',
            kind: 'number',
            label: 'Statistics Period (days)',
            min: 1,
            max: 365,
          },
          {
            key: 'homepageJellyStatShowLibraries',
            kind: 'switch',
            label: 'Show Library Statistics',
          },
          { key: 'homepageJellyStatShowUsers', kind: 'switch', label: 'Show User Statistics' },
          {
            key: 'homepageJellyStatShowMostWatched',
            kind: 'switch',
            label: 'Show Most Watched Content',
          },
          {
            key: 'homepageJellyStatShowRecentActivity',
            kind: 'switch',
            label: 'Show Recent Activity',
          },
          {
            key: 'homepageJellyStatMaxItems',
            kind: 'number',
            label: 'Maximum Items to Display',
            min: 5,
            max: 50,
          },
        ],
      },
      {
        id: 'most-watched',
        title: 'Most watched',
        entries: [
          {
            key: 'homepageJellyStatShowMostWatchedMovies',
            kind: 'switch',
            label: 'Show Most Watched Movies with Posters',
          },
          {
            key: 'homepageJellyStatShowMostWatchedShows',
            kind: 'switch',
            label: 'Show Most Watched TV Shows with Posters',
          },
          {
            key: 'homepageJellyStatShowMostListenedMusic',
            kind: 'switch',
            label: 'Show Most Listened Music with Cover Art',
          },
          {
            key: 'homepageJellyStatMostWatchedCount',
            kind: 'number',
            label: 'Number of Most Watched Items to Display',
            min: 1,
            max: 50,
          },
        ],
      },
      {
        id: 'iframe-mode',
        title: 'iFrame mode',
        entries: [
          {
            key: 'homepageJellyStatIframeHeight',
            kind: 'number',
            label: 'Iframe Height (pixels)',
            min: 300,
            max: 2000,
          },
          {
            key: 'homepageJellyStatIframeScrolling',
            kind: 'switch',
            label: 'Allow Scrolling in Iframe',
          },
        ],
      },
    ],
  },
};
