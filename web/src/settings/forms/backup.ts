// The page Backup of System Settings: the settings above the list.

import type { FormPage } from '../model';

export const BACKUP: FormPage = {
  id: 'backup',
  title: 'Backup',
  sections: [
    {
      id: 'auto-backup',
      title: 'Automatic backup',
      entries: [
        {
          kind: 'notice',
          tone: 'info',
          title: 'Scheduled jobs',
          body: 'The server runs the jobs of the schedules itself. No cron file is needed.',
          wide: true,
        },
        { key: 'autoBackupCronEnabled', kind: 'switch', label: 'Auto-Backup Organizr' },
        {
          key: 'autoBackupCronSchedule',
          kind: 'cron',
          label: 'Cron Schedule',
          help: 'You may use either Cron format or - @hourly, @daily, @monthly',
          placeholder: '* * * * *',
        },
        {
          key: 'keepBackupsCountCron',
          kind: 'number',
          label: '# Backups Keep',
          help: 'Number of backups to keep',
        },
        {
          key: 'backupLocation',
          kind: 'folder',
          label: 'Backup Save Path',
          help: 'Folder path to save Organizr Backups - Please test before saving',
          placeholder: '/path/to/folder',
          adminChanges: true,
        },
      ],
    },
  ],
};
