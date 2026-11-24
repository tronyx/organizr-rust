// The homepage item Donate: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const DONATE: HomepageItemForm = {
  id: 'donate',
  name: 'Donate',
  image: 'plugins/images/tabs/donate.png',
  category: 'Requests',
  enabledKey: 'homepageDonateEnabled',
  license: 'personal',
  page: {
    id: 'homepage-donate',
    title: 'Donate',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageDonateEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageDonateAuth',
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
          { key: 'homepageDonatePublicToken', kind: 'text', label: 'Public Token' },
          { key: 'homepageDonateSecretToken', kind: 'secret', label: 'Secret Token' },
          { key: 'homepageDonateProductID', kind: 'text', label: 'Product ID' },
        ],
      },
      {
        id: 'customize',
        title: 'Customize',
        entries: [
          { key: 'homepageDonateCustomizeHeading', kind: 'text', label: 'Heading' },
          {
            key: 'homepageDonateCustomizeDescription',
            kind: 'code',
            label: 'Description',
            wide: true,
            language: 'html',
          },
          {
            key: 'homepageDonateMinimum',
            kind: 'select',
            label: 'Minimum',
            choices: [
              { label: '1 USD', value: '100' },
              { label: '2 USD', value: '200' },
              { label: '3 USD', value: '300' },
              { label: '4 USD', value: '400' },
              { label: '5 USD', value: '500' },
              { label: '10 USD', value: '1000' },
              { label: '20 USD', value: '2000' },
              { label: '25 USD', value: '2500' },
              { label: '50 USD', value: '5000' },
              { label: '75 USD', value: '7500' },
              { label: '100 USD', value: '10000' },
            ],
          },
          {
            key: 'homepageDonateShowUserHistory',
            kind: 'switch',
            label: 'Show User Donate History',
          },
        ],
      },
      {
        id: 'about',
        title: 'About',
        entries: [
          {
            kind: 'about',
            title: 'Donations Homepage Item',
            body: 'This item allows you to use Stripe to accept donations on the homepage',
          },
        ],
      },
    ],
  },
};
