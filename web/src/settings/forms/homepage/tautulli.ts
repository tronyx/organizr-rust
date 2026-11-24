// The homepage item Tautulli: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const TAUTULLI: HomepageItemForm = {
  id: 'tautulli',
  name: 'Tautulli',
  image: 'plugins/images/tabs/tautulli.png',
  category: 'Monitor',
  enabledKey: 'homepageTautulliEnabled',
  license: 'personal',
  page: {
    id: 'homepage-tautulli',
    title: 'Tautulli',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageTautulliEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageTautulliAuth',
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
            key: 'tautulliURL',
            kind: 'tags',
            label: "Multiple URL's",
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'tautulliApikey',
            kind: 'tags',
            label: "Multiple API Key/Token's",
            hiddenValues: true,
          },
          { key: 'tautulliDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'tautulliUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/tautulli', method: 'POST' },
          },
        ],
      },
      {
        id: 'library-stats',
        title: 'Library stats',
        entries: [
          {
            key: 'tautulliLibraries',
            kind: 'switch',
            label: 'Libraries',
            help: 'Shows/hides the card with library information.',
          },
          {
            key: 'homepageTautulliLibraryAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            key: 'homepageTautulliLibraryStatsExclude',
            kind: 'tags',
            label: 'Libraries to Exclude',
          },
        ],
      },
      {
        id: 'viewing-stats',
        title: 'Viewing stats',
        entries: [
          {
            key: 'tautulliPopularMovies',
            kind: 'switch',
            label: 'Popular Movies',
            help: 'Shows/hides the card with Popular Movie information.',
          },
          {
            key: 'tautulliPopularTV',
            kind: 'switch',
            label: 'Popular TV',
            help: 'Shows/hides the card with Popular TV information.',
          },
          {
            key: 'tautulliTopMovies',
            kind: 'switch',
            label: 'Top Movies',
            help: 'Shows/hides the card with Top Movies information.',
          },
          {
            key: 'tautulliTopTV',
            kind: 'switch',
            label: 'Top TV',
            help: 'Shows/hides the card with Top TV information.',
          },
          {
            key: 'homepageTautulliViewsAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            key: 'homepageTautulliViewingStatsExclude',
            kind: 'tags',
            label: 'Libraries to Exclude',
          },
        ],
      },
      {
        id: 'other-stats',
        title: 'Other stats',
        entries: [
          {
            key: 'tautulliTopUsers',
            kind: 'switch',
            label: 'Top Users',
            help: 'Shows/hides the card with Top Users information.',
          },
          {
            key: 'tautulliTopPlatforms',
            kind: 'switch',
            label: 'Top Platforms',
            help: 'Shows/hides the card with Top Platforms information.',
          },
          {
            key: 'homepageTautulliMiscAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            key: 'tautulliFriendlyName',
            kind: 'switch',
            label: 'Use Friendly Name',
            help: 'Use the friendly name set in tautulli for users.',
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'tautulliHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'tautulliHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
          {
            key: 'homepageTautulliRefresh',
            kind: 'select',
            label: 'Refresh Seconds',
            choices: [
              { label: '2.5', value: '2500' },
              { label: '5', value: '5000' },
              { label: '10', value: '10000' },
              { label: '15', value: '15000' },
              { label: '30', value: '30000' },
              { label: '60 [1 Minute]', value: '60000' },
              { label: '300 [5 Minutes]', value: '300000' },
              { label: '600 [10 Minutes]', value: '600000' },
              { label: '900 [15 Minutes]', value: '900000' },
              { label: '1800 [30 Minutes]', value: '1800000' },
              { label: '3600 [1 Hour]', value: '3600000' },
            ],
          },
        ],
      },
      {
        id: 'socks',
        title: 'API SOCKS',
        entries: [
          { kind: 'socks', app: 'tautulli' },
          { key: 'tautulliSocksEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'tautulliSocksAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
        ],
      },
    ],
  },
};
