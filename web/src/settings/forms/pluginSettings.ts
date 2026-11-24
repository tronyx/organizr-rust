// The page Settings of Plugins: the plugin marketplace.

import type { FormPage } from '../model';

export const PLUGIN_SETTINGS: FormPage = {
  id: 'plugin-settings',
  title: 'Settings',
  sections: [
    {
      id: 'marketplace',
      title: 'Marketplace',
      entries: [
        {
          kind: 'notice',
          tone: 'danger',
          title: 'Attention',
          body: '3rd Party Repositories are not affiliated with Organizr and therefore the code on these repositories are not inspected. Use at your own risk.',
          wide: true,
        },
        {
          key: 'externalPluginMarketplaceRepos',
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
          key: 'checkForPluginUpdate',
          kind: 'switch',
          label: 'Check for Plugin Updates',
          help: 'Check for updates on page load',
        },
      ],
    },
  ],
};
