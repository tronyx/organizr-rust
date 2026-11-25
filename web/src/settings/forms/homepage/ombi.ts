// The homepage item Ombi: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const OMBI: HomepageItemForm = {
  id: 'ombi',
  name: 'Ombi',
  image: 'plugins/images/tabs/ombi.png',
  category: 'Requests',
  enabledKey: 'homepageOmbiEnabled',
  license: 'personal',
  page: {
    id: 'homepage-ombi',
    title: 'Ombi',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageOmbiEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageOmbiAuth',
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
            label: 'Ombi Fallback User',
            help: 'Organizr will request an Ombi User Token based off of this user credentials',
          },
          { key: 'ombiFallbackPassword', kind: 'secret', label: 'Ombi Fallback Password' },
          { key: 'ombiDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'ombiUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/ombi', method: 'POST' },
          },
        ],
      },
      {
        id: 'default-filter',
        title: 'Default filter',
        entries: [
          {
            key: 'ombiDefaultFilterAvailable',
            kind: 'switch',
            label: 'Show Available',
            help: 'Show All Available Ombi Requests',
          },
          {
            key: 'ombiDefaultFilterUnavailable',
            kind: 'switch',
            label: 'Show Unavailable',
            help: 'Show All Unavailable Ombi Requests',
          },
          {
            key: 'ombiDefaultFilterApproved',
            kind: 'switch',
            label: 'Show Approved',
            help: 'Show All Approved Ombi Requests',
          },
          {
            key: 'ombiDefaultFilterUnapproved',
            kind: 'switch',
            label: 'Show Unapproved',
            help: 'Show All Unapproved Ombi Requests',
          },
          {
            key: 'ombiDefaultFilterDenied',
            kind: 'switch',
            label: 'Show Denied',
            help: 'Show All Denied Ombi Requests',
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageOmbiRequestAuth',
            kind: 'select',
            label: 'Minimum Group to Request',
            source: 'groups',
          },
          {
            key: 'ombiTvDefault',
            kind: 'select',
            label: 'TV Show Default Request',
            choices: [
              { label: 'All Seasons', value: 'all' },
              { label: 'First Season Only', value: 'first' },
              { label: 'Last Season Only', value: 'last' },
            ],
          },
          { key: 'ombiLimitUser', kind: 'switch', label: 'Limit to User' },
          { key: 'ombiLimit', kind: 'number', label: 'Item Limit' },
          {
            key: 'ombiRefresh',
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
          {
            key: 'ombiAlias',
            kind: 'switch',
            label: 'Use Ombi Alias Names',
            help: 'Use Ombi Alias Names instead of Usernames - If Alias is blank, Alias will fallback to Username',
          },
        ],
      },
    ],
  },
};
