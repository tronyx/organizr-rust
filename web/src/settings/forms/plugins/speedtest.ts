// The settings of the plugin SpeedTest.

import type { FormPage } from '../../model';

export const SPEEDTEST_PLUGIN: FormPage = {
  id: 'plugin-speedtest',
  title: 'SpeedTest',
  sections: [
    {
      id: 'options',
      title: 'Options',
      entries: [
        {
          key: 'SPEEDTEST-Auth-include',
          kind: 'select',
          label: 'Minimum Authentication',
          source: 'groups',
        },
      ],
    },
  ],
};
