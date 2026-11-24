// The homepage item Monitorr: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const MONITORR: HomepageItemForm = {
  id: 'monitorr',
  name: 'Monitorr',
  image: 'plugins/images/tabs/monitorr.png',
  category: 'Monitor',
  enabledKey: 'homepageMonitorrEnabled',
  page: {
    id: 'homepage-monitorr',
    title: 'Monitorr',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageMonitorrEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageMonitorrAuth',
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
            key: 'monitorrURL',
            kind: 'text',
            label: 'URL',
            help: 'URL for Monitorr. Please use the reverse proxy URL i.e. https://domain.com/monitorr/.',
            placeholder: 'http://domain.com/monitorr/',
          },
          { key: 'monitorrDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'monitorrUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageMonitorrRefresh',
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
            key: 'monitorrCompact',
            kind: 'switch',
            label: 'Compact view',
            help: 'Toggles the compact view of this homepage module',
          },
          {
            key: 'monitorrHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'monitorrHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
        ],
      },
    ],
  },
};
