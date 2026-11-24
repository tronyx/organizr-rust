// The settings of the plugin Invites.

import type { FormPage } from '../../model';

export const INVITES_PLUGIN: FormPage = {
  id: 'plugin-invites',
  title: 'Invites',
  sections: [
    {
      id: 'back-end',
      title: 'Back end',
      entries: [
        {
          key: 'INVITES-type-include',
          kind: 'select',
          label: 'Media Server',
          choices: [
            { label: 'N/A', value: 'n/a' },
            { label: 'Plex', value: 'plex' },
            { label: 'Emby', value: 'emby' },
          ],
        },
        {
          key: 'INVITES-Auth-include',
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
        },
        {
          key: 'INVITES-allow-delete-include',
          kind: 'switch',
          label: 'Allow users to delete invites',
          help: 'This must be disabled to enforce invitation limits.',
        },
        {
          key: 'INVITES-maximum-invites',
          kind: 'number',
          label: 'Maximum number of invites permitted for users.',
          help: 'Set to 0 to disable the limit.',
          placeholder: '0',
        },
      ],
    },
    {
      id: 'plex',
      title: 'Plex',
      entries: [
        {
          key: 'plexToken',
          kind: 'secret',
          label: 'Plex Token',
          placeholder: 'Use Get Token Button',
        },
        {
          kind: 'button',
          label: 'Get Plex Token',
          text: 'Retrieve',
          action: { kind: 'plex-token' },
        },
        {
          key: 'plexID',
          kind: 'secret',
          label: 'Plex Machine',
          placeholder: 'Use Get Plex Machine Button',
        },
        {
          kind: 'button',
          label: 'Get Plex Machine',
          text: 'Retrieve',
          action: { kind: 'plex-machine' },
        },
        { key: 'INVITES-plexLibraries', kind: 'tags', label: 'Libraries' },
        {
          key: 'INVITES-plex-tv-labels',
          kind: 'text',
          label: 'TV Labels (comma separated)',
          placeholder: 'All',
        },
        {
          key: 'INVITES-plex-movies-labels',
          kind: 'text',
          label: 'Movies Labels (comma separated)',
          placeholder: 'All',
        },
        {
          key: 'INVITES-plex-music-labels',
          kind: 'text',
          label: 'Music Labels (comma separated)',
          placeholder: 'All',
        },
        {
          key: 'INVITES-add-plex-home',
          kind: 'switch',
          label: 'When user subscribe add him to Plex Home',
        },
      ],
    },
    {
      id: 'komga',
      title: 'Komga',
      entries: [
        {
          key: 'INVITES-komga-enabled',
          kind: 'switch',
          label: 'Enable Komga for auto create account',
        },
        {
          key: 'INVITES-komga-uri',
          kind: 'text',
          label: 'URL',
          placeholder: 'http(s)://hostname:port',
        },
        { key: 'INVITES-komga-api-key', kind: 'secret', label: 'Komga Api Key' },
        {
          key: 'INVITES-komga-default-user-password',
          kind: 'secret',
          label: 'Default password for new user',
        },
        {
          key: 'INVITES-komga-roles',
          kind: 'multi-select',
          label: 'Roles',
          choices: [
            { label: 'Administrator', value: 'ADMIN' },
            { label: 'File download', value: 'FILE_DOWNLOAD' },
            { label: 'Page streaming', value: 'PAGE_STREAMING' },
            { label: 'Kobo Sync', value: 'KOBO_SYNC' },
            { label: 'Koreader Sync', value: 'KOREADER_SYNC' },
          ],
        },
        {
          key: 'INVITES-komga-libraryIds',
          kind: 'multi-select',
          label: 'Libraries',
          source: 'komga-libraries',
        },
      ],
    },
    {
      id: 'nextcloud',
      title: 'Nextcloud',
      entries: [
        {
          key: 'INVITES-nextcloud-enabled',
          kind: 'switch',
          label: 'Enable Nextcloud for auto create account',
        },
        {
          key: 'INVITES-nextcloud-plex-sso',
          kind: 'switch',
          label: 'Enable if you have Plex SSO app installed on Nextcloud',
        },
        { key: 'INVITES-nextcloud-url', kind: 'text', label: 'Nextcloud URI' },
        { key: 'INVITES-nextcloud-admin-user', kind: 'text', label: 'Nextcloud Admin User' },
        {
          key: 'INVITES-nextcloud-admin-password',
          kind: 'secret',
          label: 'Nextcloud Admin Password',
        },
        {
          key: 'INVITES-nextcloud-quota',
          kind: 'text',
          label: 'Storage quota for user after subscription (empty for no-limit)',
          placeholder: '10GB',
        },
        {
          key: 'INVITES-nextcloud-groups-member',
          kind: 'multi-select',
          label: 'Nextcloud Groups after subscription',
          source: 'nextcloud-groups',
        },
      ],
    },
    {
      id: 'emby',
      title: 'Emby',
      entries: [
        {
          key: 'embyToken',
          kind: 'secret',
          label: 'Emby API key',
          placeholder: 'enter key from emby',
        },
        {
          key: 'embyURL',
          kind: 'text',
          label: 'Emby server adress',
          placeholder: 'localhost:8086',
        },
        {
          key: 'INVITES-EmbyTemplate',
          kind: 'text',
          label: 'Emby User to be used as template for new users',
          placeholder: 'AdamSmith',
        },
      ],
    },
  ],
};
