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

/** The answer of a server without a marketplace. */
const NO_MARKET = { status: 409, message: 'No marketplace is set up' };

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
    // The approval names the file, the permissions and the level that the dialog showed.
    expect(sent.find((request) => request.method === 'POST')?.body).toEqual({
      sha256: 'ab'.repeat(32),
      permissions: ['storage'],
      least_access: 4,
    });
    expect((await screen.findByRole('table')).textContent).toContain('Switched on');
    expect(screen.getByRole('link', { name: 'Open' }).getAttribute('href')).toContain(
      'api/v2/extension/hello/',
    );
  });

  it('names each site that an extension asks to reach', async () => {
    stubApi({
      'GET extensions': {
        data: [row({ permissions: ['http-local', 'http:api.example.com', 'storage'] })],
      },
    });
    render(ExtensionsEditor, { ondone: vi.fn() });

    await fireEvent.click(await screen.findByRole('button', { name: 'Approve Hello' }));
    const permissions = screen.getByRole('list', { name: 'Permissions of the extension' });
    expect(permissions.textContent).toContain('Send requests to the site api.example.com');
    expect(permissions.textContent).toContain('also when they are on your local network');
    expect(permissions.textContent).toContain('Keep values of its own');
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
    const missing = { status: 404, message: 'Not Found' };
    const sent = stubApi({ 'GET extensions': missing, 'GET extensions/marketplace': missing });
    const { container } = render(ExtensionsEditor, { ondone: () => {} });
    await waitFor(() => expect(sent).toHaveLength(2));
    expect(container.textContent?.trim()).toBe('');
  });

  it('lists the marketplace and installs an extension of it', async () => {
    let installed = false;
    const offer = (changes: Record<string, unknown>) => ({
      id: 'hello',
      name: 'Hello',
      version: '1.2.0',
      description: 'From the marketplace.',
      author: 'Sam',
      installed: false,
      up_to_date: false,
      problem: null,
      ...changes,
    });
    const sent = stubApi({
      'GET extensions': () => ({ data: installed ? [row()] : [] }),
      'GET extensions/marketplace': () => ({
        data: [
          offer({ installed, up_to_date: installed }),
          offer({ id: 'greedy', name: 'Greedy', problem: 'It asks for too much' }),
        ],
      }),
      'POST extensions/marketplace/hello': () => {
        installed = true;
        return { message: 'Extension installed' };
      },
    });
    const ondone = vi.fn();
    render(ExtensionsEditor, { ondone });

    const table = await screen.findByRole('table', { name: 'Extensions of the marketplace' });
    expect(table.textContent).toContain('Not installed');
    expect(table.textContent).toContain('It asks for too much');
    // An entry with a problem has no button.
    expect(screen.queryByRole('button', { name: 'Install Greedy' })).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'Install Hello' }));
    await waitFor(() =>
      expect(ondone).toHaveBeenCalledWith(
        'The extension Hello is installed. Approve it in the list above.',
      ),
    );
    expect(sent.some((request) => request.path === 'extensions/marketplace/hello')).toBe(true);
    // The new extension waits for the approval, and the marketplace offers no install.
    expect(await screen.findByRole('button', { name: 'Approve Hello' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Install Hello' })).toBeNull();
  });

  it('shows no marketplace when none is set up, and a warning when it gave no list', async () => {
    stubApi({
      'GET extensions': { data: [] },
      'GET extensions/marketplace': { status: 409, message: 'No marketplace is set up' },
    });
    const { unmount } = render(ExtensionsEditor, { ondone: () => {} });
    await screen.findByText('There are no extensions in the folder.');
    expect(screen.queryByText('Marketplace')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    unmount();

    stubApi({
      'GET extensions': { data: [] },
      'GET extensions/marketplace': { status: 409, message: 'Could not access the marketplace' },
    });
    render(ExtensionsEditor, { ondone: () => {} });
    expect((await screen.findByRole('alert')).textContent).toContain(
      'Could not access the marketplace',
    );
  });

  it('says so when the folder is empty', async () => {
    stubApi({ 'GET extensions': { data: [] }, 'GET extensions/marketplace': NO_MARKET });
    render(ExtensionsEditor, { ondone: () => {} });
    expect(await screen.findByText('There are no extensions in the folder.')).toBeTruthy();
  });
});
