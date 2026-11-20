import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { FieldGroup } from '../../lib/settingsForm';
import { stubApi } from '../../test-helpers';
import SettingsForm from './SettingsForm.svelte';

const GROUPS: FieldGroup[] = [
  {
    title: 'Sonarr',
    test: 'sonarr',
    fields: [{ key: 'sonarrURL', label: 'Address of Sonarr', kind: 'text' }],
  },
  {
    title: 'Calendar',
    fields: [{ key: 'calendarDefault', label: 'View of the calendar', kind: 'text' }],
  },
];

const CONFIG = { data: { sonarrURL: 'http://sonarr.example.com', calendarDefault: 'month' } };

describe('the connection test of a group of settings', () => {
  it('asks the server and shows that the connection works', async () => {
    const sent = stubApi({
      'GET config': CONFIG,
      'POST test/sonarr': { message: 'API Connection succeeded' },
    });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });

    const button = await screen.findByRole('button', { name: 'Test the connection of Sonarr' });
    // Only a group with a test has the button.
    expect(screen.queryByRole('button', { name: 'Test the connection of Calendar' })).toBeNull();
    await fireEvent.click(button);

    expect((await screen.findByRole('status')).textContent).toContain('API Connection succeeded');
    expect(
      sent.some((request) => request.method === 'POST' && request.path === 'test/sonarr'),
    ).toBe(true);
  });

  it('has a text of its own when the server sends none', async () => {
    stubApi({ 'GET config': CONFIG, 'POST test/sonarr': {} });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    await fireEvent.click(await screen.findByRole('button', { name: /Test the connection/ }));
    expect((await screen.findByRole('status')).textContent).toContain('The connection works.');
  });

  it('shows why the test failed', async () => {
    stubApi({
      'GET config': CONFIG,
      'POST test/sonarr': { status: 500, message: 'http://sonarr.example.com: Unauthorized' },
    });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    await fireEvent.click(await screen.findByRole('button', { name: /Test the connection/ }));
    expect((await screen.findByRole('alert')).textContent).toContain('Unauthorized');
  });

  it('waits for the save when the form has changes, because the test reads what is stored', async () => {
    const sent = stubApi({ 'GET config': CONFIG });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    const button = (await screen.findByRole('button', {
      name: /Test the connection/,
    })) as HTMLButtonElement;
    expect(button.disabled).toBe(false);

    await fireEvent.input(screen.getByLabelText('Address of Sonarr'), {
      target: { value: 'http://other.example.com' },
    });
    expect(button.disabled).toBe(true);
    expect(screen.getByText('Save the settings before you test.')).toBeTruthy();
    expect(sent.filter((request) => request.method === 'POST')).toHaveLength(0);
  });
});

const BACKUP_GROUPS: FieldGroup[] = [
  {
    title: 'Backups',
    scheduler: true,
    fields: [
      { key: 'autoBackupCronSchedule', label: 'Schedule', kind: 'text', test: 'cron' },
      { key: 'backupLocation', label: 'Folder', kind: 'text', test: 'folder' },
    ],
  },
];

const BACKUP_CONFIG = { data: { autoBackupCronSchedule: '@weekly', backupLocation: '' } };

describe('the tests of the value of a field', () => {
  it('send the schedule in the form as one JSON text, also before a save', async () => {
    const sent = stubApi({
      'GET config': BACKUP_CONFIG,
      'POST test/cron': { message: 'Schedule was validated' },
    });
    render(SettingsForm, { groups: BACKUP_GROUPS, ondone: () => {} });
    await fireEvent.input(await screen.findByLabelText('Schedule'), {
      target: { value: '30 3 * * *' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test the schedule' }));

    expect((await screen.findByRole('status')).textContent).toContain('Schedule was validated');
    expect(sent.find((request) => request.path === 'test/cron')?.body).toBe('30 3 * * *');
  });

  it('send the folder in the field folder, and show why it failed', async () => {
    const sent = stubApi({
      'GET config': BACKUP_CONFIG,
      'POST test/folder': {
        status: 409,
        message: 'Folder path is not valid or permissions insufficient',
      },
    });
    render(SettingsForm, { groups: BACKUP_GROUPS, ondone: () => {} });
    await fireEvent.input(await screen.findByLabelText('Folder'), {
      target: { value: '/backups' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test the folder' }));

    expect((await screen.findByRole('alert')).textContent).toContain('permissions insufficient');
    expect(sent.find((request) => request.path === 'test/folder')?.body).toEqual({
      folder: '/backups',
    });
  });
});

describe('the check of the scheduler', () => {
  it('asks the server whether the scheduler runs', async () => {
    const sent = stubApi({
      'GET config': BACKUP_CONFIG,
      'GET test/cron': { message: 'Cron file is setup' },
    });
    render(SettingsForm, { groups: BACKUP_GROUPS, ondone: () => {} });
    await fireEvent.click(
      await screen.findByRole('button', { name: 'Check that the scheduler runs' }),
    );

    expect((await screen.findByRole('status')).textContent).toContain('Cron file is setup');
    expect(sent.some((request) => request.method === 'GET' && request.path === 'test/cron')).toBe(
      true,
    );
  });

  it('is only in a group that asks for it', async () => {
    stubApi({ 'GET config': CONFIG });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    await screen.findByRole('button', { name: /Test the connection/ });
    expect(screen.queryByRole('button', { name: 'Check that the scheduler runs' })).toBeNull();
    expect(screen.queryByRole('button', { name: /Test the (schedule|folder)/ })).toBeNull();
  });
});
