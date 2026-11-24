// The homepage item Jackett: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const JACKETT: HomepageItemForm = {
  id: 'jackett',
  name: 'Jackett',
  image: 'plugins/images/tabs/jackett.png',
  category: 'Utility',
  enabledKey: 'homepageJackettEnabled',
  page: {
    id: 'homepage-jackett',
    title: 'Jackett',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageJackettEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageJackettAuth',
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
            key: 'jackettURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'jackettToken', kind: 'secret', label: 'API Key/Token' },
          { key: 'jackettDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'jackettUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/jackett', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageJackettBackholeDownload',
            kind: 'switch',
            label: 'Prefer black hole download',
            help: 'Prefer black hole download link instead of direct/magnet download',
          },
        ],
      },
    ],
  },
};
