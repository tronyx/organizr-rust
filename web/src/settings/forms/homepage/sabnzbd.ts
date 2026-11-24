// The homepage item SabNZBD: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const SABNZBD: HomepageItemForm = {
  id: 'sabnzbd',
  name: 'SabNZBD',
  image: 'plugins/images/tabs/sabnzbd.png',
  category: 'Downloader',
  enabledKey: 'homepageSabnzbdEnabled',
  license: 'personal',
  page: {
    id: 'homepage-sabnzbd',
    title: 'SabNZBD',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageSabnzbdEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageSabnzbdAuth',
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
            key: 'sabnzbdURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'sabnzbdToken', kind: 'secret', label: 'API Key/Token' },
          { key: 'sabnzbdDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'sabnzbdUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/sabnzbd', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'sabnzbdRefresh',
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
          { key: 'sabnzbdCombine', kind: 'switch', label: 'Add to Combined Downloader' },
        ],
      },
      {
        id: 'socks',
        title: 'API SOCKS',
        entries: [
          { kind: 'socks', app: 'sabnzbd', wide: true },
          { key: 'sabnzbdSocksEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'sabnzbdSocksAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
        ],
      },
    ],
  },
};
