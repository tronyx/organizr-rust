// The homepage item Overseerr: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const OVERSEERR: HomepageItemForm = {
  id: 'overseerr',
  name: 'Overseerr',
  image: 'plugins/images/tabs/overseerr.png',
  category: 'Requests',
  enabledKey: 'homepageOverseerrEnabled',
  license: 'personal',
  page: {
    id: 'homepage-overseerr',
    title: 'Overseerr',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageOverseerrEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageOverseerrAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          {
            kind: 'notice',
            tone: 'info',
            title: 'Attention',
            body: 'Since Organizr supports multiple Request Providers, You must now select which service you want to submit requests through',
          },
          {
            key: 'defaultRequestService',
            kind: 'select',
            label: 'Default Request Service',
            choices: [
              { label: 'Ombi', value: 'ombi' },
              { label: 'Overseerr', value: 'overseerr' },
            ],
          },
        ],
      },
      {
        id: 'connection',
        title: 'Connection',
        entries: [
          {
            key: 'overseerrURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          { key: 'overseerrToken', kind: 'secret', label: 'API Key/Token' },
          {
            key: 'overseerrFallbackUser',
            kind: 'text',
            label: 'Overseerr Fallback User',
            help: 'Organizr will request an Overseerr User Token based off of this user credentials',
          },
          {
            key: 'overseerrFallbackPassword',
            kind: 'secret',
            label: 'Overseerr Fallback Password',
          },
          { key: 'overseerrDisableCertCheck', kind: 'switch', label: 'Disable Certificate Check' },
          { key: 'overseerrUseCustomCertificate', kind: 'switch', label: 'Use Custom Certificate' },
          {
            kind: 'button',
            label: 'Test Connection',
            help: 'Remember! Please save before using the test button!',
            text: 'Test Connection',
            action: { kind: 'test', path: 'test/overseerr', method: 'POST' },
          },
        ],
      },
      {
        id: 'default-filter',
        title: 'Default filter',
        entries: [
          {
            key: 'overseerrDefaultFilterAvailable',
            kind: 'switch',
            label: 'Show Available',
            help: 'Show All Available Overseerr Requests',
          },
          {
            key: 'overseerrDefaultFilterUnavailable',
            kind: 'switch',
            label: 'Show Unavailable',
            help: 'Show All Unavailable Overseerr Requests',
          },
          {
            key: 'overseerrDefaultFilterApproved',
            kind: 'switch',
            label: 'Show Approved',
            help: 'Show All Approved Overseerr Requests',
          },
          {
            key: 'overseerrDefaultFilterUnapproved',
            kind: 'switch',
            label: 'Show Unapproved',
            help: 'Show All Unapproved Overseerr Requests',
          },
          {
            key: 'overseerrDefaultFilterDenied',
            kind: 'switch',
            label: 'Show Denied',
            help: 'Show All Denied Overseerr Requests',
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'homepageOverseerrRequestAuth',
            kind: 'select',
            label: 'Minimum Group to Request',
            source: 'groups',
          },
          {
            key: 'overseerrTvDefault',
            kind: 'select',
            label: 'TV Show Default Request',
            choices: [
              { label: 'All Seasons', value: 'all' },
              { label: 'First Season Only', value: 'first' },
              { label: 'Last Season Only', value: 'last' },
              { label: 'Let User Select', value: 'user' },
            ],
          },
          { key: 'overseerrLimitUser', kind: 'switch', label: 'Limit to User' },
          { key: 'overseerrLimit', kind: 'number', label: 'Item Limit' },
          { key: 'overseerrPrefer4K', kind: 'switch', label: 'Prefer 4K Server' },
          {
            key: 'overseerrRefresh',
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
