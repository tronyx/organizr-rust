// The page Updates of System Settings.

import type { FormPage } from '../model';

export const UPDATES: FormPage = {
  id: 'updates',
  title: 'Updates',
  sections: [
    {
      id: 'check',
      title: 'Update check',
      entries: [
        {
          key: 'checkForUpdate',
          kind: 'switch',
          label: 'Check For Update',
          help: 'Check for update on Organizr load',
        },
        {
          key: 'branch',
          kind: 'select',
          label: 'Branch',
          help: 'Since you are using the Official Docker image, Change the image to change the branch',
          disabled: true,
          choices: [
            { label: 'Develop', value: 'v2-develop' },
            { label: 'Master', value: 'v2-master' },
          ],
        },
        {
          kind: 'button',
          label: 'Force Install Branch',
          help: 'Since you are using the official Docker image, you can just restart your Docker container to update Organizr',
          text: 'Retrieve',
          action: {
            kind: 'none',
            why: 'This application runs as a container image: pull a new image.',
          },
        },
      ],
    },
    {
      id: 'auto-update',
      title: 'Automatic updates',
      entries: [
        {
          kind: 'notice',
          tone: 'info',
          title: 'Scheduled jobs',
          body: 'The server runs the jobs of the schedules itself. No cron file is needed.',
          wide: true,
        },
        { key: 'autoUpdateCronEnabled', kind: 'switch', label: 'Auto-Update Organizr' },
        {
          key: 'autoUpdateCronSchedule',
          kind: 'cron',
          label: 'Cron Schedule',
          help: 'You may use either Cron format or - @hourly, @daily, @monthly',
          placeholder: '* * * * *',
        },
      ],
    },
  ],
};
