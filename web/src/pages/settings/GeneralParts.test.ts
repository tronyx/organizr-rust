import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import CustomCertificate from './CustomCertificate.svelte';
import DatabaseJournal from './DatabaseJournal.svelte';
import UpdateCheck from './UpdateCheck.svelte';

describe('DatabaseJournal', () => {
  it('shows the mode on SQLite and sets the other one', async () => {
    let mode = 'wal';
    const sent = stubApi({
      'GET status': { data: { action: 'launch', database: 'sqlite' } },
      'GET database/journal': () => ({ data: { journal_mode: mode } }),
      'PUT database/journal/DELETE': () => {
        mode = 'delete';
        return { message: 'Journal Mode updated to: DELETE', data: { journal_mode: mode } };
      },
    });
    const ondone = vi.fn();
    render(DatabaseJournal, { ondone });

    expect((await screen.findByText(/uses the journal mode/)).textContent).toContain('WAL');
    expect(screen.getByRole('button', { name: 'Set WAL mode' })).toBeDisabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Set DELETE mode' }));

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The journal mode is DELETE.'));
    expect(screen.getByText(/uses the journal mode/).textContent).toContain('DELETE');
    expect(screen.getByRole('button', { name: 'Set DELETE mode' })).toBeDisabled();
    expect(sent.filter((request) => request.method === 'PUT')).toHaveLength(1);
  });

  it('shows nothing on PostgreSQL', async () => {
    const sent = stubApi({ 'GET status': { data: { action: 'launch', database: 'postgresql' } } });
    render(DatabaseJournal, { ondone: () => {} });
    await vi.waitFor(() => expect(sent).toHaveLength(1));
    expect(screen.queryByRole('heading', { name: 'Journal mode of the database' })).toBeNull();
  });

  it('shows why the server refused a change', async () => {
    stubApi({
      'GET status': { data: { action: 'launch', database: 'sqlite' } },
      'GET database/journal': { data: { journal_mode: 'wal' } },
      'PUT database/journal/DELETE': { status: 409, message: 'The database is in use' },
    });
    render(DatabaseJournal, { ondone: () => {} });
    await fireEvent.click(await screen.findByRole('button', { name: 'Set DELETE mode' }));
    expect((await screen.findByRole('alert')).textContent).toContain('The database is in use');
  });
});

describe('UpdateCheck', () => {
  it('asks for the newest version and says what it found', async () => {
    stubApi({
      'GET config': { data: { checkForUpdate: true } },
      'GET update/check': {
        data: { current: '0.1.0', latest: '0.2.0', updateAvailable: true, releaseUrl: null },
      },
    });
    const ondone = vi.fn();
    render(UpdateCheck, { ondone });

    await fireEvent.click(screen.getByRole('button', { name: 'Check for updates now' }));

    expect(await screen.findByText(/The newest release is version 0.2.0/)).toBeTruthy();
    expect(screen.getByText(/runs version 0.1.0/)).toBeTruthy();
    expect(ondone).toHaveBeenCalledWith('Version 0.2.0 is out.');
  });

  it('shows why the check failed', async () => {
    stubApi({
      'GET config': { data: { checkForUpdate: true } },
      'GET update/check': {
        status: 500,
        message: 'The newest version could not be read from GitHub',
      },
    });
    render(UpdateCheck, { ondone: () => {} });
    await fireEvent.click(screen.getByRole('button', { name: 'Check for updates now' }));
    expect((await screen.findByRole('alert')).textContent).toContain('could not be read');
  });
});

describe('CustomCertificate', () => {
  it('uploads the chosen file in the field file', async () => {
    let field: unknown = null;
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_address: string, init: RequestInit = {}) => {
        if ((init.method ?? 'GET') === 'GET') {
          const loaded = field !== null;
          const status = { loaded, certificates: loaded ? 1 : 0 };
          return new Response(
            JSON.stringify({ response: { result: 'success', message: null, data: status } }),
            { status: 200 },
          );
        }
        field = (init.body as FormData).get('file');
        const envelope = {
          response: { result: 'success', message: 'ca.pem has been uploaded', data: null },
        };
        return new Response(JSON.stringify(envelope), { status: 200 });
      }),
    );
    const ondone = vi.fn();
    render(CustomCertificate, { ondone });
    expect((await screen.findByTestId('certificate-status')).textContent).toContain(
      'Custom Certificate not found - please upload below',
    );
    const file = new File(['-----BEGIN CERTIFICATE-----'], 'ca.pem');
    await fireEvent.change(screen.getByLabelText('Certificate file'), {
      target: { files: [file] },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Upload the certificate' }));

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('ca.pem has been uploaded'));
    expect((field as File).name).toBe('ca.pem');
    // The status is loaded again after the upload.
    await vi.waitFor(() =>
      expect(screen.getByTestId('certificate-status').textContent).toContain(
        'Custom Certificate Loaded',
      ),
    );
    expect(screen.getByTestId('certificate-status').textContent).toContain('(1 certificate)');
  });

  it('asks for a file, and shows why the server refused one', async () => {
    stubApi({
      'POST certificate/custom': {
        status: 422,
        message: 'A certificate of the file is not that of a certificate authority',
      },
    });
    render(CustomCertificate, { ondone: () => {} });
    await fireEvent.click(screen.getByRole('button', { name: 'Upload the certificate' }));
    expect((await screen.findByRole('alert')).textContent).toContain('Choose a file first.');

    await fireEvent.change(screen.getByLabelText('Certificate file'), {
      target: { files: [new File(['x'], 'leaf.pem')] },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Upload the certificate' }));
    await vi.waitFor(() =>
      expect(screen.getByRole('alert').textContent).toContain('not that of a certificate'),
    );
  });
});
