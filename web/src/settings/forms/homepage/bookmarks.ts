// The homepage item Bookmarks: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const BOOKMARKS: HomepageItemForm = {
  id: 'bookmarks',
  name: 'Bookmarks',
  image: 'plugins/images/bookmark.png',
  category: 'Links',
  enabledKey: 'homepageBookmarksEnabled',
  page: {
    id: 'homepage-bookmarks',
    title: 'Bookmarks',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          {
            key: 'homepageBookmarksEnabled',
            kind: 'switch',
            label: 'Enable',
            help: 'Toggles the view module for Bookmarks',
          },
          {
            key: 'homepageBookmarksAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageBookmarksRefresh',
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
        ],
      },
    ],
  },
};
