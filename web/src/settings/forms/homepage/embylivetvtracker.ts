// The homepage item EmbyLiveTVTracker: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const EMBYLIVETVTRACKER: HomepageItemForm = {
  id: 'embylivetvtracker',
  name: 'EmbyLiveTVTracker',
  image: 'plugins/images/homepage/embyLiveTVTracker.png',
  category: 'Media Server',
  enabledKey: 'homepageEmbyLiveTVTrackerEnabled',
  license: 'personal',
  page: {
    id: 'homepage-embylivetvtracker',
    title: 'EmbyLiveTVTracker',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageEmbyLiveTVTrackerEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageEmbyLiveTVTrackerAuth',
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
            key: 'embyURL',
            adminChanges: { key: 'authBackend', values: ['emby_local', 'emby_all'] },
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'embyToken', kind: 'secret', label: 'API Key/Token' },
          { key: 'embyDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'embyUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/embyLiveTVTracker', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageEmbyLiveTVTrackerRefresh',
            kind: 'number',
            label: 'Auto-refresh Interval (minutes)',
            min: 1,
            max: 60,
          },
          {
            key: 'homepageEmbyLiveTVTrackerCompactView',
            kind: 'switch',
            label: 'Use Compact View',
          },
          {
            key: 'homepageEmbyLiveTVTrackerShowDuration',
            kind: 'switch',
            label: 'Show Recording Duration',
          },
          {
            key: 'homepageEmbyLiveTVTrackerShowSeriesInfo',
            kind: 'switch',
            label: 'Show Series Information',
          },
          {
            key: 'homepageEmbyLiveTVTrackerShowUserInfo',
            kind: 'switch',
            label: 'Show User Information',
          },
          {
            key: 'homepageEmbyLiveTVTrackerMaxItems',
            kind: 'number',
            label: 'Maximum Scheduled Items',
            min: 5,
            max: 50,
          },
          {
            key: 'homepageEmbyLiveTVTrackerShowCompleted',
            kind: 'switch',
            label: 'Show Completed Recordings',
          },
          {
            key: 'homepageEmbyLiveTVTrackerDaysShown',
            kind: 'number',
            label: 'Days of Completed Recordings',
            min: 1,
            max: 30,
          },
          {
            key: 'homepageEmbyLiveTVTrackerMaxCompletedItems',
            kind: 'number',
            label: 'Maximum Completed Items',
            min: 5,
            max: 50,
          },
        ],
      },
    ],
  },
};
