// The settings of the plugin Plex Libraries.

import type { FormPage } from '../../model';

export const PLEX_LIBRARIES_PLUGIN: FormPage = {
  id: 'plugin-plexlibraries',
  title: 'Plex Libraries',
  sections: [
    {
      id: 'about',
      title: 'About',
      entries: [
        {
          kind: 'notice',
          tone: 'info',
          title: 'Plex Libraries',
          body: 'Lets each person switch the libraries of the Plex server that the own account sees, and lets Admins and Co-Admins switch them for every account the server is shared with. The plugin uses the Plex token and the Plex ID of System Settings, Sign-in. Make the tab of its page in the section Tab.',
          wide: true,
        },
      ],
    },
    {
      id: 'options',
      title: 'Options',
      entries: [
        {
          key: 'PLEXLIBRARIES-Auth-include',
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
        },
        {
          key: 'PLEXLIBRARIES-librariesToInclude',
          kind: 'multi-select',
          label: 'Libraries that users may switch',
          help: 'Without a library, users see their libraries but change none.',
          source: 'plex-libraries',
        },
      ],
    },
    {
      id: 'tab',
      title: 'Tab',
      entries: [{ kind: 'part', id: 'plex-libraries-tab' }],
    },
  ],
};
