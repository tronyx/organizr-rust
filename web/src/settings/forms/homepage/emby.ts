// The homepage item Emby: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const EMBY: HomepageItemForm = {
  id: 'emby',
  name: 'Emby',
  image: 'plugins/images/tabs/emby.png',
  category: 'Media Server',
  enabledKey: 'homepageEmbyEnabled',
  license: 'personal',
  page: {
    id: 'homepage-emby',
    title: 'Emby',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageEmbyEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageEmbyAuth',
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
            key: 'embyURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'embyToken', kind: 'secret', label: 'API Key/Token' },
          { key: 'embyDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'embyUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/emby', method: 'POST' },
          },
        ],
      },
      {
        id: 'active-streams',
        title: 'Active streams',
        entries: [
          { key: 'homepageEmbyStreams', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageEmbyStreamsAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          { key: 'homepageShowStreamNames', kind: 'switch', label: 'User Information' },
          {
            key: 'homepageShowStreamNamesAuth',
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
        ],
      },
      {
        id: 'recent-items',
        title: 'Recent items',
        entries: [
          { key: 'homepageEmbyRecent', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageEmbyRecentAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
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
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageEmbyLink',
            kind: 'text',
            label: 'Emby Homepage Link URL',
            help: 'Available variables: {id} {serverId}',
          },
          {
            key: 'embyTabName',
            kind: 'text',
            label: 'Emby Tab Name',
            placeholder: 'Only use if you have Emby in a reverse proxy',
          },
          {
            key: 'embyTabURL',
            kind: 'text',
            label: 'Emby Tab WAN URL',
            placeholder: 'Only use if you have Emby in a reverse proxy',
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
        ],
      },
    ],
  },
};
