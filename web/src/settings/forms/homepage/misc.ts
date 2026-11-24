// The homepage item Misc: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const MISC: HomepageItemForm = {
  id: 'misc',
  name: 'Misc',
  image: 'plugins/images/organizr/logo-no-border.png',
  category: 'Custom',
  enabledKey: '',
  page: {
    id: 'homepage-misc',
    title: 'Misc',
    sections: [
      {
        id: 'youtube',
        title: 'YouTube',
        entries: [
          {
            key: 'youtubeAPI',
            kind: 'secret',
            label: 'Youtube API Key',
            help: 'Please make sure to input this API key as the organizr one gets limited',
          },
        ],
      },
    ],
  },
};
