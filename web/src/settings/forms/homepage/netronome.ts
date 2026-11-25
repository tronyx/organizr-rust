// The homepage item Netronome: its card and the page of its settings. PHP Organizr has
// no such item.

import type { HomepageItemForm } from '../../model';

export const NETRONOME: HomepageItemForm = {
  id: 'netronome',
  name: 'Netronome',
  image: 'plugins/images/tabs/netronome.png',
  category: 'Monitor',
  enabledKey: 'homepageNetronomeEnabled',
  page: {
    id: 'homepage-netronome',
    title: 'Netronome',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageNetronomeEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageNetronomeAuth',
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
            key: 'netronomeURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'netronomeDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'netronomeUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageNetronomeTimeRange',
            kind: 'select',
            label: 'Time Range',
            help: 'The speed tests that the average and the limits are taken from',
            choices: [
              { label: 'Last 24 Hours', value: '24h' },
              { label: 'Last 3 Days', value: '3d' },
              { label: 'Last Week', value: 'week' },
              { label: 'Last Month', value: 'month' },
            ],
          },
          {
            key: 'homepageNetronomeRefresh',
            kind: 'select',
            label: 'Refresh Seconds',
            choices: [
              { label: '60 [1 Minute]', value: '60000' },
              { label: '300 [5 Minutes]', value: '300000' },
              { label: '600 [10 Minutes]', value: '600000' },
              { label: '900 [15 Minutes]', value: '900000' },
              { label: '1800 [30 Minutes]', value: '1800000' },
              { label: '3600 [1 Hour]', value: '3600000' },
            ],
          },
          {
            key: 'homepageNetronomeHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'homepageNetronomeHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
        ],
      },
    ],
  },
};
