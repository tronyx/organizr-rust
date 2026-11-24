import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { parseRoute } from '../../lib/route.svelte';
import { session } from '../../lib/session.svelte';
import type { Launch } from '../../lib/types';
import { stubApi } from '../../test-helpers';
import BackupsEditor from './BackupsEditor.svelte';

const NAME = 'backup[2026-11-20_03-15 - ab][0.1.0].zip';

function listOf(names: string[]) {
  return {
    data: {
      total_size: '1.50 MB',
      total_files: names.length,
      files:
        names.length === 0
          ? null
          : names.map((name) => ({ name, size: '2 MB', date: '2026-11-20T03:15:00Z' })),
    },
  };
}

/** The settings of the backups, as `GET /config` sends them. */
const CONFIG = {
  data: {
    autoBackupCronEnabled: true,
    autoBackupCronSchedule: '@daily',
    keepBackupsCountCron: '7',
    backupLocation: '/data/backups',
  },
};

/** Signs in a person of the group with the given level. */
function signIn(groupID: number, group: string) {
  session.launch = { user: { groupID, group, loggedin: true } } as unknown as Launch;
}

describe('BackupsEditor', () => {
  beforeEach(() => signIn(0, 'Admin'));
  afterEach(() => {
    session.launch = null;
  });

  it('tells a group below Co-Admin that it cannot use the backups, and asks nothing', async () => {
    signIn(2, 'Super User');
    const sent = stubApi({});
    render(BackupsEditor, { ondone: () => {} });

    expect((await screen.findByRole('status')).textContent).toContain(
      'Only an Admin or a Co-Admin can make, download and delete backups',
    );
    expect(screen.getByRole('status').textContent).toContain('signed in as Super User');
    expect(screen.queryByRole('button', { name: 'Make a backup now' })).toBeNull();
    expect(sent).toEqual([]);
  });

  it('lets a Co-Admin use the backups, as PHP does', async () => {
    signIn(1, 'Co-Admin');
    stubApi({ 'GET config': CONFIG, 'GET backup': listOf([NAME]) });
    render(BackupsEditor, { ondone: () => {} });

    expect(await screen.findByRole('table', { name: 'Backups' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Make a backup now' })).toBeTruthy();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('lists the backups with a link that saves the file', async () => {
    stubApi({ 'GET config': CONFIG, 'GET backup': listOf([NAME]) });
    render(BackupsEditor, { ondone: () => {} });

    const table = await screen.findByRole('table', { name: 'Backups' });
    expect(table.textContent).toContain(NAME);
    expect(table.textContent).toContain('2026-11-20 03:15 UTC');
    expect(screen.getByText('1 backups, 1.50 MB in all.')).toBeTruthy();
    const link = within(table).getByRole('link', { name: `Save ${NAME} to this computer` });
    expect(link.getAttribute('href')).toContain(`api/v2/backup/${encodeURIComponent(NAME)}`);
    expect(link.getAttribute('download')).toBe(NAME);
  });

  it('makes a backup and shows the new list', async () => {
    const names: string[] = [];
    const sent = stubApi({
      'GET config': CONFIG,
      'GET backup': () => listOf(names),
      'POST backup': () => {
        names.push(NAME);
        return { message: 'Backup has been created' };
      },
    });
    const ondone = vi.fn();
    render(BackupsEditor, { ondone });
    expect(await screen.findByText('There are no backups yet.')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Make a backup now' }));

    expect((await screen.findByRole('table')).textContent).toContain(NAME);
    expect(ondone).toHaveBeenCalledWith('The backup is made.');
    expect(sent.filter((request) => request.method === 'POST')).toHaveLength(1);
  });

  it('deletes a backup by its name', async () => {
    const names = [NAME];
    const sent = stubApi({
      'GET config': CONFIG,
      'GET backup': () => listOf(names),
      [`DELETE backup/${encodeURIComponent(NAME)}`]: () => {
        names.length = 0;
        return { message: `${NAME} has been deleted` };
      },
    });
    const ondone = vi.fn();
    render(BackupsEditor, { ondone });

    await fireEvent.click(await screen.findByRole('button', { name: `Delete ${NAME}` }));

    expect(await screen.findByText('There are no backups yet.')).toBeTruthy();
    expect(ondone).toHaveBeenCalledWith('The backup is deleted.');
    expect(sent.some((request) => request.method === 'DELETE')).toBe(true);
  });

  it('shows why the server refused', async () => {
    stubApi({ 'GET config': CONFIG, 'GET backup': { status: 401, message: 'Not Authorized' } });
    render(BackupsEditor, { ondone: () => {} });
    expect((await screen.findByRole('alert')).textContent).toContain('Not Authorized');
    expect(screen.queryByRole('button', { name: 'Make a backup now' })).toBeNull();
  });

  it('shows_the_backup_settings_above_the_list_and_saves_them', async () => {
    const sent = stubApi({
      'GET config': CONFIG,
      'GET backup': listOf([NAME]),
      'PUT config': { message: 'Config items updated' },
      'GET launch': { data: { user: { groupID: 0, group: 'Admin', loggedin: true } } },
    });
    const ondone = vi.fn();
    render(BackupsEditor, { ondone });

    const table = await screen.findByRole('table', { name: 'Backups' });
    const heading = await screen.findByRole('heading', { name: 'Backup', level: 2 });
    expect(heading.compareDocumentPosition(table) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByLabelText('Auto-Backup Organizr')).toBeChecked();
    expect(screen.getByLabelText('Backup Save Path')).toHaveValue('/data/backups');

    await fireEvent.input(screen.getByLabelText('Backup Save Path'), {
      target: { value: '/srv/backups' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));
    expect(sent.filter((request) => request.method === 'PUT').map((r) => r.body)).toEqual([
      { backupLocation: '/srv/backups' },
    ]);
  });

  it('has an address of its own in the settings', () => {
    expect(parseRoute('#/settings/backups')).toEqual({ page: 'settings', section: 'backups' });
  });
});
