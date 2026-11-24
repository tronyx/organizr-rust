// The homepage item Pi-hole: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const PI_HOLE: HomepageItemForm = {
  id: 'pi-hole',
  name: 'Pi-hole',
  image: 'plugins/images/tabs/pihole.png',
  category: 'Monitor',
  enabledKey: 'homepagePiholeEnabled',
  page: {
    id: 'homepage-pi-hole',
    title: 'Pi-hole',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepagePiholeEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepagePiholeAuth',
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
            key: 'piholeURL',
            kind: 'tags',
            label: "Multiple URL's",
            help: 'Please make sure to use local IP address and port at the end of the URL. You can add multiple Pi-holes by comma separating the URLs.',
            placeholder: 'http(s)://hostname:port/',
          },
          {
            key: 'piholeToken',
            kind: 'tags',
            label: "Multiple API Key/Token's",
            hiddenValues: true,
          },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/pihole', method: 'POST' },
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'piholeHeaderToggle',
            kind: 'switch',
            label: 'Toggle Title',
            help: 'Shows/hides the title of this homepage module',
          },
          {
            key: 'homepagePiholeCombine',
            kind: 'switch',
            label: 'Combine stat cards',
            help: 'This controls whether to combine the stats for multiple pihole instances into 1 card.',
          },
        ],
      },
    ],
  },
};
