// The page SSO of System Settings: single sign-on to other services.

import type { FormPage } from '../model';

export const SSO: FormPage = {
  id: 'sso',
  title: 'SSO',
  sections: [
    {
      id: 'plex',
      title: 'Plex',
      entries: [
        { key: 'plexToken', kind: 'secret', label: 'API Key/Token' },
        {
          kind: 'button',
          label: 'Get Plex Token',
          text: 'Retrieve',
          action: { kind: 'plex-token' },
        },
        { key: 'plexID', kind: 'text', label: 'Plex Machine ID', masked: true },
        {
          kind: 'button',
          label: 'Get Plex Machine',
          text: 'Retrieve',
          action: { kind: 'plex-machine' },
        },
        {
          key: 'plexAdmin',
          kind: 'text',
          label: 'Plex Admin Username or Email',
          adminChanges: true,
        },
        { key: 'ssoPlex', kind: 'switch', label: 'Enable', adminChanges: true },
      ],
    },
    {
      id: 'tautulli',
      title: 'Tautulli',
      entries: [
        {
          key: 'tautulliURL',
          adminChanges: { key: 'ssoTautulli', values: [true] },
          kind: 'tags',
          label: "Multiple URL's",
          help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
          placeholder: 'http(s)://hostname:port',
        },
        {
          key: 'ssoTautulliAuth',
          adminChanges: true,
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
        },
        { key: 'ssoTautulli', kind: 'switch', label: 'Enable', adminChanges: true },
      ],
    },
    {
      id: 'overseerr',
      title: 'Overseerr',
      entries: [
        {
          key: 'overseerrURL',
          adminChanges: { key: 'ssoOverseerr', values: [true] },
          kind: 'text',
          label: 'URL',
          help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
          placeholder: 'http(s)://hostname:port',
        },
        { key: 'overseerrToken', kind: 'secret', label: 'API Key/Token' },
        {
          key: 'overseerrFallbackUser',
          kind: 'text',
          label: 'Overseerr Fallback Email',
          help: 'DO NOT SET THIS TO YOUR ADMIN ACCOUNT. We recommend you create a local account as a "catch all" for when Organizr is unable to perform SSO. Organizr will request a User Token based off of this user credentials',
        },
        { key: 'overseerrFallbackPassword', kind: 'secret', label: 'Overseerr Fallback Password' },
        { key: 'ssoOverseerr', kind: 'switch', label: 'Enable', adminChanges: true },
      ],
    },
    {
      id: 'petio',
      title: 'Petio',
      entries: [
        {
          key: 'petioURL',
          adminChanges: { key: 'ssoPetio', values: [true] },
          kind: 'text',
          label: 'URL',
          help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
          placeholder: 'http(s)://hostname:port',
        },
        { key: 'petioToken', kind: 'secret', label: 'API Key/Token' },
        {
          key: 'petioFallbackUser',
          kind: 'text',
          label: 'Petio Fallback Email',
          help: 'DO NOT SET THIS TO YOUR ADMIN ACCOUNT. We recommend you create a local account as a "catch all" for when Organizr is unable to perform SSO. Organizr will request a User Token based off of this user credentials',
        },
        { key: 'petioFallbackPassword', kind: 'secret', label: 'Petio Fallback Password' },
        { key: 'ssoPetio', kind: 'switch', label: 'Enable', adminChanges: true },
      ],
    },
    {
      id: 'ombi',
      title: 'Ombi',
      entries: [
        {
          key: 'ombiURL',
          adminChanges: { key: 'ssoOmbi', values: [true] },
          kind: 'text',
          label: 'URL',
          help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
          placeholder: 'http(s)://hostname:port',
        },
        { key: 'ombiToken', kind: 'secret', label: 'API Key/Token' },
        {
          key: 'ombiFallbackUser',
          kind: 'text',
          label: 'Ombi Fallback Email',
          help: 'DO NOT SET THIS TO YOUR ADMIN ACCOUNT. We recommend you create a local account as a "catch all" for when Organizr is unable to perform SSO. Organizr will request a User Token based off of this user credentials',
        },
        { key: 'ombiFallbackPassword', kind: 'secret', label: 'Ombi Fallback Password' },
        { key: 'ssoOmbi', kind: 'switch', label: 'Enable', adminChanges: true },
      ],
    },
    {
      id: 'jellyfin',
      title: 'Jellyfin',
      entries: [
        {
          key: 'jellyfinURL',
          adminChanges: [
            { key: 'authBackend', values: ['jellyfin'] },
            { key: 'ssoJellyfin', values: [true] },
          ],
          kind: 'text',
          label: 'Jellyfin API URL',
          help: 'Please make sure to use the local address to the API',
          placeholder: 'http(s)://hostname:port',
        },
        {
          key: 'jellyfinSSOURL',
          adminChanges: { key: 'ssoJellyfin', values: [true] },
          kind: 'text',
          label: 'Jellyfin SSO URL',
          help: "Please make sure to use the same (sub)domain to access Jellyfin as Organizr's",
          placeholder: 'http(s)://hostname:port',
        },
        { key: 'ssoJellyfin', kind: 'switch', label: 'Enable', adminChanges: true },
      ],
    },
    {
      id: 'komga',
      title: 'Komga',
      entries: [
        {
          key: 'komgaURL',
          adminChanges: { key: 'ssoKomga', values: [true] },
          kind: 'text',
          label: 'URL',
          help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
          placeholder: 'http(s)://hostname:port',
        },
        {
          key: 'ssoKomgaAuth',
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
          adminChanges: true,
        },
        { key: 'ssoKomga', kind: 'switch', label: 'Enable', adminChanges: true },
        {
          key: 'komgaFallbackUser',
          kind: 'text',
          label: 'Komga Fallback Email',
          help: 'DO NOT SET THIS TO YOUR ADMIN ACCOUNT. We recommend you create a local account as a "catch all" for when Organizr is unable to perform SSO. Organizr will request a User Token based off of this user credentials',
        },
        { key: 'komgaFallbackPassword', kind: 'secret', label: 'Komga Fallback Password' },
        {
          key: 'komgaSSOMasterPassword',
          kind: 'secret',
          label: 'Komga Master Password',
          help: 'Sets master password if using oAuth backend - This will set the password on the login form for logins using oAuth where no password is supplied.',
        },
      ],
    },
  ],
};
