// The homepage item Speedtest: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const SPEEDTEST: HomepageItemForm = {
  id: 'speedtest',
  name: 'Speedtest',
  image: 'plugins/images/tabs/speedtest-icon.png',
  category: 'Monitor',
  enabledKey: 'homepageSpeedtestEnabled',
  page: {
    id: 'homepage-speedtest',
    title: 'Speedtest',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageSpeedtestEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageSpeedtestAuth',
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
            key: 'speedtestURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'speedtestDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'speedtestUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'speedtestHeader',
            kind: 'text',
            label: 'Title',
            help: 'Sets the title of this homepage module',
          },
          {
            key: 'speedtestHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
        ],
      },
    ],
  },
};
