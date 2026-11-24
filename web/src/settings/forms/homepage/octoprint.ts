// The homepage item Octoprint: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const OCTOPRINT: HomepageItemForm = {
  id: 'octoprint',
  name: 'Octoprint',
  image: 'plugins/images/tabs/octoprint.png',
  category: 'Monitor',
  enabledKey: 'homepageOctoprintEnabled',
  page: {
    id: 'homepage-octoprint',
    title: 'Octoprint',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageOctoprintEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageOctoprintAuth',
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
            key: 'octoprintURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'octoprintToken', kind: 'secret', label: 'API Key/Token' },
          { key: 'octoprintDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'octoprintUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'octoprintHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'octoprintHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
        ],
      },
    ],
  },
};
