// The homepage item UptimeKuma: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const UPTIMEKUMA: HomepageItemForm = {
  id: 'uptimekuma',
  name: 'UptimeKuma',
  image: 'plugins/images/tabs/kuma.png',
  category: 'Monitor',
  enabledKey: 'homepageUptimeKumaEnabled',
  page: {
    id: 'homepage-uptimekuma',
    title: 'UptimeKuma',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageUptimeKumaEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageUptimeKumaAuth',
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
            key: 'uptimeKumaURL',
            kind: 'text',
            label: 'URL',
            help: 'URL for Uptime Kuma e.g. http://kuma:3001 (no trailing slash)',
            placeholder: 'http://kuma:3001',
          },
          { key: 'uptimeKumaToken', kind: 'secret', label: 'API Key/Token' },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageUptimeKumaRefresh',
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
          {
            key: 'homepageUptimeKumaHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'homepageUptimeKumaHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
          {
            key: 'homepageUptimeKumaCompact',
            kind: 'switch',
            label: 'Compact view',
            help: 'Toggles the compact view of this homepage module',
          },
          { key: 'homepageUptimeKumaShowLatency', kind: 'switch', label: 'Show monitor latency' },
        ],
      },
    ],
  },
};
