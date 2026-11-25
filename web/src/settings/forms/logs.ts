// The page Logs of System Settings: the settings above the log viewer.

import type { FormPage } from '../model';

export const LOGS: FormPage = {
  id: 'logs',
  title: 'Logs',
  sections: [
    {
      id: 'logs',
      title: 'Logs',
      entries: [
        {
          key: 'logLevel',
          kind: 'select',
          label: 'Log Level',
          choices: [
            { label: 'Debug', value: 'DEBUG' },
            { label: 'Info', value: 'INFO' },
            { label: 'Notice', value: 'NOTICE' },
            { label: 'Warning', value: 'WARNING' },
            { label: 'Error', value: 'ERROR' },
            { label: 'Critical', value: 'CRITICAL' },
            { label: 'Alert', value: 'ALERT' },
            { label: 'Emergency', value: 'EMERGENCY' },
          ],
        },
        {
          key: 'includeDatabaseQueriesInDebug',
          kind: 'switch',
          label: 'Include Database Queries',
          help: 'Include Database queries in debug logs',
        },
        {
          key: 'maxLogFiles',
          kind: 'number',
          label: 'Maximum Log Files',
          help: 'Number of log files to preserve',
        },
        {
          key: 'logLiveUpdateRefresh',
          kind: 'select',
          label: 'Live Update Refresh',
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
          key: 'logPageSize',
          kind: 'select',
          label: 'Log Page Size',
          choices: [
            { label: '10 Items', value: '10' },
            { label: '25 Items', value: '25' },
            { label: '50 Items', value: '50' },
            { label: '100 Items', value: '100' },
          ],
        },
      ],
    },
    {
      id: 'slack',
      title: 'Slack',
      entries: [
        {
          key: 'sendLogsToSlack',
          kind: 'switch',
          label: 'Send Logs to Slack',
          help: 'Send Logs to Slack as well',
        },
        {
          key: 'slackLogLevel',
          kind: 'select',
          label: 'Slack Log Level',
          choices: [
            { label: 'Debug', value: 'DEBUG' },
            { label: 'Info', value: 'INFO' },
            { label: 'Notice', value: 'NOTICE' },
            { label: 'Warning', value: 'WARNING' },
            { label: 'Error', value: 'ERROR' },
            { label: 'Critical', value: 'CRITICAL' },
            { label: 'Alert', value: 'ALERT' },
            { label: 'Emergency', value: 'EMERGENCY' },
          ],
        },
        {
          key: 'slackLogWebhook',
          kind: 'secret',
          label: 'Slack Webhook URL',
          help: 'If using Discord make sure to end the URL with /slack',
          placeholder: 'http(s)://hostname:port',
        },
        {
          key: 'slackLogWebHookChannel',
          kind: 'text',
          label: 'Slack Channel for Webhook',
          help: 'Channel ID for webhook - Not needed for Discord',
        },
        {
          kind: 'button',
          label: 'Test Slack',
          help: 'Test only sends a warning message so make sure Slack Log Level is Warning when testing',
          text: 'Test Slack',
          action: { kind: 'test', path: 'test/slack-logs', method: 'POST' },
        },
      ],
    },
  ],
};
