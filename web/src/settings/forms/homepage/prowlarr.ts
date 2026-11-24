// The homepage item Prowlarr: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const PROWLARR: HomepageItemForm = {
  id: 'prowlarr',
  name: 'Prowlarr',
  image: 'plugins/images/tabs/prowlarr.png',
  category: 'Utility',
  enabledKey: 'homepageProwlarrEnabled',
  page: {
    id: 'homepage-prowlarr',
    title: 'Prowlarr',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageProwlarrEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageProwlarrAuth',
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
            key: 'prowlarrURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'prowlarrToken', kind: 'secret', label: 'API Key/Token' },
          { key: 'prowlarrDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'prowlarrUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/prowlarr', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageProwlarrBackholeDownload',
            kind: 'switch',
            label: 'Prefer black hole download',
            help: 'Prefer black hole download link instead of direct/magnet download',
          },
        ],
      },
    ],
  },
};
