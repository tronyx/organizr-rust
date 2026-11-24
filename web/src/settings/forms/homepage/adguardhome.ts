// The homepage item AdGuardHome: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const ADGUARDHOME: HomepageItemForm = {
  id: 'adguardhome',
  name: 'AdGuardHome',
  image: 'plugins/images/tabs/AdGuardHome.png',
  category: 'Monitor',
  enabledKey: 'homepageAdGuardEnabled',
  page: {
    id: 'homepage-adguardhome',
    title: 'AdGuardHome',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageAdGuardEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageAdGuardAuth',
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
            key: 'adguardURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port. You can add multiple AdGuard Homes by comma separating the URLs.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'adGuardUsername', kind: 'text', label: 'Username' },
          { key: 'adGuardPassword', kind: 'secret', label: 'Password' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/adguard', method: 'POST' },
          },
        ],
      },
      {
        id: 'stats',
        title: 'Stats',
        entries: [
          { key: 'adguardQueriesToggle', kind: 'switch', label: 'Total Queries' },
          { key: 'adguardQueriesBlockedToggle', kind: 'switch', label: 'Queries Blocked' },
          { key: 'adguardPercentToggle', kind: 'switch', label: 'Percent Blocked' },
          { key: 'adguardProcessingToggle', kind: 'switch', label: 'Processing Time' },
          { key: 'adguardDomainListToggle', kind: 'switch', label: 'Domains on Blocklist' },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'adguardHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
          {
            key: 'homepageAdGuardCombine',
            kind: 'switch',
            label: 'Combine stat cards',
            help: 'This controls whether to combine the stats for multiple adguard instances into 1 card.',
          },
        ],
      },
    ],
  },
};
