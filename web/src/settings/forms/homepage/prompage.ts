// The homepage item PromPage: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const PROMPAGE: HomepageItemForm = {
  id: 'prompage',
  name: 'PromPage',
  image: 'plugins/images/tabs/prompage.png',
  category: 'Monitor',
  enabledKey: 'homepagePromPageEnabled',
  page: {
    id: 'homepage-prompage',
    title: 'PromPage',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [{ key: 'homepagePromPageEnabled', kind: 'switch', label: 'Enable' }],
      },
      {
        id: 'connection',
        title: 'Connection',
        entries: [
          {
            key: 'promPageURL',
            kind: 'text',
            label: 'URL',
            help: 'URL for PromPage e.g. http://prompage:3000 (no trailing slash)',
            placeholder: 'http://prompage:3000',
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepagePromPageRefresh',
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
            key: 'homepagePromPageHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'homepagePromPageHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
          {
            key: 'homepagePromPageCompact',
            kind: 'switch',
            label: 'Compact view',
            help: 'Toggles the compact view of this homepage module',
          },
          { key: 'homepagePromPageShowUptime', kind: 'switch', label: 'Show monitor uptime' },
        ],
      },
    ],
  },
};
