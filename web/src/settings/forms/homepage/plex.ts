// The homepage item Plex: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const PLEX: HomepageItemForm = {
  id: 'plex',
  name: 'Plex',
  image: 'plugins/images/tabs/plex.png',
  category: 'Media Server',
  enabledKey: 'homepagePlexEnabled',
  license: 'personal',
  page: {
    id: 'homepage-plex',
    title: 'Plex',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepagePlexEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepagePlexAuth',
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
            key: 'plexURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'plexDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'plexUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          { key: 'plexToken', kind: 'secret', label: 'API Key/Token' },
          {
            kind: 'button',
            label: 'Get Plex Token',
            text: 'Retrieve',
            action: { kind: 'plex-token' },
          },
          { key: 'plexID', kind: 'text', label: 'Plex Machine' },
          {
            kind: 'button',
            label: 'Get Plex Machine',
            text: 'Retrieve',
            action: { kind: 'plex-machine' },
          },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/plex', method: 'POST' },
          },
        ],
      },
      {
        id: 'active-streams',
        title: 'Active streams',
        entries: [
          { key: 'homepagePlexStreams', kind: 'switch', label: 'Enable' },
          {
            key: 'homepagePlexStreamsAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            key: 'homepageShowStreamNames',
            kind: 'switch',
            label: 'User Information',
            help: 'Show user and IP information',
          },
          {
            key: 'homepageShowStreamNamesAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            key: 'homepageShowStreamNamesWithoutIp',
            kind: 'switch',
            label: 'User Information Without IP',
            help: 'Only shows username and no IP information',
          },
          {
            key: 'homepageShowStreamNamesWithoutIpAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            key: 'homepageStreamRefresh',
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
          { key: 'homepagePlexStreamsExclude', kind: 'tags', label: 'Libraries to Exclude' },
        ],
      },
      {
        id: 'recent-items',
        title: 'Recent items',
        entries: [
          { key: 'homepagePlexRecent', kind: 'switch', label: 'Enable' },
          {
            key: 'homepagePlexRecentAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          { key: 'homepagePlexRecentExclude', kind: 'tags', label: 'Libraries to Exclude' },
          { key: 'homepageRecentLimit', kind: 'number', label: 'Item Limit' },
          {
            key: 'homepageRecentRefresh',
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
      {
        id: 'media-search',
        title: 'Media search',
        entries: [
          { key: 'mediaSearch', kind: 'switch', label: 'Enable' },
          {
            key: 'mediaSearchAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          { key: 'homepagePlexSearchExclude', kind: 'tags', label: 'Libraries to Exclude' },
          {
            key: 'mediaSearchType',
            kind: 'select',
            label: 'Media Search Server',
            choices: [
              { label: 'N/A', value: '' },
              { label: 'Plex', value: 'plex' },
              { label: 'Emby [Not Available]', value: 'emby' },
            ],
          },
        ],
      },
      {
        id: 'playlists',
        title: 'Playlists',
        entries: [
          { key: 'homepagePlexPlaylist', kind: 'switch', label: 'Enable' },
          {
            key: 'homepagePlexPlaylistAuth',
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
            key: 'plexTabName',
            kind: 'text',
            label: 'Plex Tab Name',
            placeholder: 'Only use if you have Plex in a reverse proxy',
          },
          {
            key: 'plexTabURL',
            kind: 'text',
            label: 'Plex Tab WAN URL',
            placeholder: 'http(s)://domain.com/plex',
          },
          {
            key: 'cacheImageSize',
            kind: 'select',
            label: 'Image Cache Quality',
            choices: [
              { label: 'Low', value: '.5' },
              { label: '1x', value: '1' },
              { label: '2x', value: '2' },
              { label: '3x', value: '3' },
            ],
          },
          {
            key: 'homepageUseCustomStreamNames',
            kind: 'switch',
            label: 'Use Tautulli custom names for users',
          },
        ],
      },
    ],
  },
};
