// The homepage item UniFi: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const UNIFI: HomepageItemForm = {
  id: 'unifi',
  name: 'UniFi',
  image: 'plugins/images/tabs/unifi.png',
  category: 'Monitor',
  enabledKey: 'homepageUnifiEnabled',
  page: {
    id: 'homepage-unifi',
    title: 'UniFi',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageUnifiEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageUnifiAuth',
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
            key: 'unifiURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'unifiDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'unifiUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            key: 'unifiUsername',
            kind: 'text',
            label: 'Username',
            help: 'Username is case-sensitive',
          },
          { key: 'unifiPassword', kind: 'secret', label: 'Password' },
          {
            key: 'unifiSiteName',
            kind: 'text',
            label: 'Site Name (Not for UnifiOS)',
            help: 'Site Name - not Site ID nor Site Description',
          },
          {
            kind: 'button',
            label: 'Grab Unifi Site (Not for UnifiOS)',
            text: 'Get Unifi Site',
            action: { kind: 'lookup', path: 'test/unifi/site' },
          },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/unifi', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageUnifiRefresh',
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
