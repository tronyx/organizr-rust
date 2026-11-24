// The page General of System Settings.

import type { FormPage } from '../model';

export const GENERAL: FormPage = {
  id: 'general',
  title: 'General',
  sections: [
    {
      id: 'api',
      title: 'API key',
      entries: [
        { key: 'organizrAPI', kind: 'secret-copy', label: 'Organizr API' },
        {
          kind: 'button',
          label: 'Generate New API Key',
          text: 'Generate',
          action: { kind: 'new-key', key: 'organizrAPI' },
        },
        {
          kind: 'notice',
          tone: 'info',
          title: 'API Documentation',
          body: "The documentation for Organizr's API is included with this installation.",
          wide: true,
        },
      ],
    },
    {
      id: 'settings-page',
      title: 'Default settings page',
      entries: [
        {
          key: 'defaultSettingsTab',
          kind: 'select',
          label: 'Default Settings Tab',
          help: 'Choose which Settings Tab to be default when opening settings page',
          choices: [
            { label: 'Tab Editor', value: '0' },
            { label: 'Customize', value: '1' },
            { label: 'User Management', value: '2' },
            { label: 'Image Manager', value: '3' },
            { label: 'Plugins', value: '4' },
            { label: 'System Settings', value: '5' },
          ],
        },
      ],
    },
    {
      id: 'debug',
      title: 'Debug',
      entries: [
        {
          key: 'socksDebug',
          kind: 'switch',
          label: 'Enable Debug Log',
          help: 'Enable the option to have socks output to logs',
        },
        {
          key: 'maxSocksDebugSize',
          kind: 'number',
          label: 'Max Debug Data Rows',
          help: 'Max amount of rows in debug log',
        },
      ],
    },
    {
      id: 'certificate',
      title: 'Certificate',
      entries: [{ kind: 'part', id: 'certificate' }],
    },
    {
      id: 'database',
      title: 'Database',
      entries: [{ kind: 'part', id: 'database' }],
    },
  ],
};
