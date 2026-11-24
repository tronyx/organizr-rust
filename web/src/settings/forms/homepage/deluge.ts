// The homepage item Deluge: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const DELUGE: HomepageItemForm = {
  id: 'deluge',
  name: 'Deluge',
  image: 'plugins/images/tabs/deluge.png',
  category: 'Downloader',
  enabledKey: 'homepageDelugeEnabled',
  license: 'personal',
  page: {
    id: 'homepage-deluge',
    title: 'Deluge',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageDelugeEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageDelugeAuth',
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
            key: 'delugeURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'delugePassword',
            kind: 'secret',
            label: 'Password',
            help: 'Note that using a blank password might not work correctly.',
          },
          { key: 'delugeDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'delugeUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/deluge', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          { key: 'delugeHideSeeding', kind: 'switch', label: 'Hide Seeding' },
          { key: 'delugeHideCompleted', kind: 'switch', label: 'Hide Completed' },
          { key: 'delugeHideStatus', kind: 'switch', label: 'Hide Status' },
          { key: 'delugeCombine', kind: 'switch', label: 'Add to Combined Downloader' },
          {
            key: 'delugeRefresh',
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
    ],
  },
};
