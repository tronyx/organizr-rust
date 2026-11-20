import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { session } from '../../lib/session.svelte';
import { stubApi } from '../../test-helpers';
import ExtensionsEditor from './ExtensionsEditor.svelte';

function launchFor(groupID: number) {
  return {
    data: {
      user: {
        username: 'zed',
        group: 'Admin',
        groupID,
        email: null,
        image: null,
        userID: 1,
        loggedin: true,
        locked: 0,
      },
      tabs: [],
      categories: [],
      plugins: { includes: {} },
      appearance: { title: 'Organizr' },
      settings: {
        sso: { plex: { oAuthEnabled: false } },
        homepage: { refresh: {}, options: {} },
        misc: {},
        ping: {},
      },
      status: { status: 'ok' },
    },
  };
}

function row(changes: Record<string, unknown> = {}) {
  return {
    id: 'hello',
    name: 'Hello',
    version: '1.0.0',
    description: 'Greets the caller.',
    author: 'Sam',
    permissions: ['storage'],
    least_access: 4,
    sha256: 'ab'.repeat(32),
    enabled: false,
    approved: false,
    changed: false,
    problem: null,
    ...changes,
  };
}

async function asGroup(groupID: number) {
  stubApi({ 'GET launch': launchFor(groupID) });
  await session.load();
}

describe('ExtensionsEditor', () => {
  beforeEach(() => asGroup(0));

  it('asks the Admin to approve an extension with what it can do', async () => {
    let enabled = false;
    const sent = stubApi({
      'GET extensions': () => ({ data: [row({ enabled, approved: enabled })] }),
      'POST extensions/hello/enable': () => {
        enabled = true;
        return { message: 'Extension enabled' };
      },
    });
    const ondone = vi.fn();
    render(ExtensionsEditor, { ondone });

    const table = await screen.findByRole('table', { name: 'Extensions' });
    expect(table.textContent).toContain('Not approved');
    expect(table.textContent).toContain('Groups of the level 4 and above');
    expect(within(table).queryByRole('link', { name: 'Open' })).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'Approve Hello' }));
    const permissions = screen.getByRole('list', { name: 'Permissions of the extension' });
    expect(permissions.textContent).toContain('Keep values of its own');
    expect(screen.getByText(/SHA-256 of the file: abab/)).toBeTruthy();
    // Nothing is sent before the Admin says yes.
    expect(sent.filter((request) => request.method === 'POST')).toHaveLength(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Approve and switch on' }));
    await waitFor(() =>
      expect(ondone).toHaveBeenCalledWith('The extension Hello is approved and switched on.'),
    );
    // The approval names the file and the permissions that the dialog showed.
    expect(sent.find((request) => request.method === 'POST')?.body).toEqual({
      sha256: 'ab'.repeat(32),
      permissions: ['storage'],
    });
    expect((await screen.findByRole('table')).textContent).toContain('Switched on');
    expect(screen.getByRole('link', { name: 'Open' }).getAttribute('href')).toContain(
      'api/v2/extension/hello/',
    );
  });

  it('switches an extension off and deletes one', async () => {
    const sent = stubApi({
      'GET extensions': { data: [row({ enabled: true, approved: true })] },
      'POST extensions/hello/disable': { message: 'Extension disabled' },
      'DELETE extensions/hello': { message: 'Extension deleted' },
    });
    const ondone = vi.fn();
    render(ExtensionsEditor, { ondone });

    await fireEvent.click(await screen.findByRole('button', { name: 'Switch Hello off' }));
    await waitFor(() =>
      expect(ondone).toHaveBeenCalledWith('The extension Hello is switched off.'),
    );
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Hello' }));
    await waitFor(() => expect(ondone).toHaveBeenCalledWith('The extension Hello is deleted.'));
    expect(sent.map((request) => request.method)).toContain('DELETE');
  });

  it('says what is wrong with an extension, and offers no approval for it', async () => {
    stubApi({
      'GET extensions': {
        data: [
          row({ id: 'broken', name: 'Broken', problem: 'The file plugin.wasm cannot be read' }),
          row({ id: 'old', name: 'Old', changed: true }),
        ],
      },
    });
    render(ExtensionsEditor, { ondone: () => {} });

    const table = await screen.findByRole('table');
    expect(table.textContent).toContain('The file plugin.wasm cannot be read');
    expect(table.textContent).toContain('The file changed after the approval');
    expect(screen.queryByRole('button', { name: 'Approve Broken' })).toBeNull();
    // A changed file can be approved again.
    expect(screen.getByRole('button', { name: 'Approve Old' })).toBeTruthy();
  });

  it('shows the list to a Co-Admin without the buttons that change it', async () => {
    await asGroup(1);
    stubApi({ 'GET extensions': { data: [row({ enabled: true, approved: true })] } });
    render(ExtensionsEditor, { ondone: () => {} });

    await screen.findByRole('table');
    expect(screen.getByRole('link', { name: 'Open' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: /Switch|Approve|Delete/ })).toBeNull();
  });

  it('shows nothing on a server that was built without extensions', async () => {
    const sent = stubApi({ 'GET extensions': { status: 404, message: 'Not Found' } });
    const { container } = render(ExtensionsEditor, { ondone: () => {} });
    await waitFor(() => expect(sent).toHaveLength(1));
    expect(container.textContent?.trim()).toBe('');
  });

  it('says so when the folder is empty', async () => {
    stubApi({ 'GET extensions': { data: [] } });
    render(ExtensionsEditor, { ondone: () => {} });
    expect(await screen.findByText('There are no extensions in the folder.')).toBeTruthy();
  });
});
