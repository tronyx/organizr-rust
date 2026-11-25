// The Marketplace Settings page of Customize: the theme marketplace.

import type { FormPage } from '../model';

export const THEME_SETTINGS: FormPage = {
  id: 'theme-settings',
  title: 'Marketplace Settings',
  sections: [
    {
      id: 'marketplace',
      title: 'Marketplace',
      entries: [
        {
          kind: 'notice',
          tone: 'danger',
          title: 'Attention',
          body: '3rd Party Repositories are not affiliated with Organizr and therefore the themes on these repositories are not inspected. Use at your own risk.',
          wide: true,
        },
        {
          key: 'externalThemeMarketplaceRepos',
          kind: 'tags',
          label: 'External Marketplace Repo',
          help: 'Only supports Github repos',
          placeholder: 'http(s)://hostname:port',
          wide: true,
        },
        {
          key: 'githubAccessToken',
          kind: 'secret',
          label: 'Github Person Access Token',
          help: 'The Github Person Access Token will help with API rate limiting as well as let you access your own Private Repos',
        },
        {
          key: 'checkForThemeUpdate',
          kind: 'switch',
          label: 'Check for Theme Updates',
          help: 'Check for updates on page load',
        },
      ],
    },
  ],
};
