// The settings of the plugin HealthChecks.

import type { FormPage } from '../../model';

export const HEALTHCHECKS_PLUGIN: FormPage = {
  id: 'plugin-healthchecks',
  title: 'HealthChecks',
  sections: [
    {
      id: 'schedule',
      title: 'Schedule',
      entries: [
        {
          kind: 'notice',
          tone: 'info',
          title: 'Scheduled jobs',
          body: 'The server runs the jobs of the schedules itself. No cron file is needed.',
          wide: true,
        },
        { key: 'HEALTHCHECKS-cron-run-enabled', kind: 'switch', label: 'Enable' },
        {
          key: 'HEALTHCHECKS-cron-run-schedule',
          kind: 'cron',
          label: 'Cron Schedule',
          help: 'You may use either Cron format or - @hourly, @daily, @monthly',
          placeholder: '* * * * *',
        },
      ],
    },
    {
      id: 'options',
      title: 'Options',
      entries: [
        {
          key: 'HEALTHCHECKS-Auth-include',
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
        },
        {
          key: 'HEALTHCHECKS-PingURL',
          kind: 'text',
          label: 'URL',
          help: 'URL for HealthChecks Ping',
          placeholder: 'HealthChecks Ping URL',
        },
        { key: 'HEALTHCHECKS-401-enabled', kind: 'switch', label: '401 Error as Success' },
        { key: 'HEALTHCHECKS-403-enabled', kind: 'switch', label: '403 Error as Success' },
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
          help: 'URL for HealthChecks API',
          placeholder: 'HealthChecks API URL',
        },
        { key: 'healthChecksToken', kind: 'secret', label: 'Token' },
      ],
    },
    {
      id: 'services',
      title: 'Services',
      entries: [{ kind: 'part', id: 'health-check-services' }],
    },
  ],
};
