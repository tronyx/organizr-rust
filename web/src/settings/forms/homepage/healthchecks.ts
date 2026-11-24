// The homepage item HealthChecks: its card and the page of its settings.

import type { HomepageItemForm } from '../../model';

export const HEALTHCHECKS: HomepageItemForm = {
  id: 'healthchecks',
  name: 'HealthChecks',
  image: 'plugins/images/tabs/healthchecks.png',
  category: 'Monitor',
  enabledKey: 'homepageHealthChecksEnabled',
  page: {
    id: 'homepage-healthchecks',
    title: 'HealthChecks',
    sections: [
      {
        id: 'enable',
        title: 'Enable',
        entries: [
          { key: 'homepageHealthChecksEnabled', kind: 'switch', label: 'Enable' },
          {
            key: 'homepageHealthChecksAuth',
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
            key: 'healthChecksURL',
            kind: 'text',
            label: 'URL',
            help: 'Please make sure to use local IP address and port - You also may use local dns name too.',
            placeholder: 'http(s)://hostname:port',
          },
          {
            key: 'healthChecksToken',
            kind: 'tags',
            label: "Multiple API Key/Token's",
            hiddenValues: true,
          },
          {
            key: 'healthChecksDisableCertCheck',
            kind: 'switch',
            label: 'Disable Certificate Check',
          },
          {
            key: 'healthChecksUseCustomCertificate',
            kind: 'switch',
            label: 'Use Custom Certificate',
          },
        ],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'healthChecksTags',
            kind: 'tags',
            label: 'Tags',
            help: 'Pull only checks with this tag - Blank for all',
            placeholder: 'Multiple tags using CSV - tag1,tag2',
          },
          {
            key: 'homepageHealthChecksRefresh',
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
          { key: 'homepageHealthChecksShowDesc', kind: 'switch', label: 'Show Description' },
          { key: 'homepageHealthChecksShowTags', kind: 'switch', label: 'Show Tags' },
        ],
      },
    ],
  },
};
