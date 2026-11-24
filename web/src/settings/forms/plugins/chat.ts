// The settings of the plugin Chat.

import type { FormPage } from '../../model';

export const CHAT_PLUGIN: FormPage = {
  id: 'plugin-chat',
  title: 'Chat',
  sections: [
    {
      id: 'options',
      title: 'Options',
      entries: [
        {
          key: 'CHAT-Auth-include',
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
        },
        { key: 'CHAT-messageLoadLimit', kind: 'number', label: '# of Previous Messages' },
      ],
    },
  ],
};
