import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { LogRecord } from '../../lib/logs';
import { parseRoute } from '../../lib/route.svelte';
import { stubApi } from '../../test-helpers';
import LogViewer from './LogViewer.svelte';

function record(message: string, level = 'INFO', trace = message): LogRecord {
  return {
    log_level: level,
    message,
    channel: 'Login',
    username: 'zoe',
    trace_id: trace,
    request_id: null,
    file: 'crates/api/src/handlers/login.rs',
    line: 117,
    context: {},
    remote_ip_address: '192.0.2.7',
    datetime: '2026-11-20 03:15:00.123456',
    timezone: 'UTC',
  };
}

function pageOf(
  records: LogRecord[],
  files = ['2026-11-20', '2026-11-19'],
  total = records.length,
) {
  return {
    data: {
      pageInfo: { results: total, totalPages: Math.ceil(total / 25), pageSize: '25', page: 1 },
      results: records,
      files,
    },
  };
}

const CONFIG = { data: { logPageSize: '25', logLiveUpdateRefresh: '2500' } };

/** The query of each request to the log, in order. */
function queries(): URLSearchParams[] {
  const fetched = vi.mocked(fetch).mock.calls.map(([address]) => new URL(String(address)));
  return fetched
    .filter((address) => address.pathname.includes('/api/v2/log/'))
    .map((address) => address.searchParams);
}

afterEach(() => {
  vi.useRealTimers();
});

describe('LogViewer', () => {
  it('shows the records of the newest file with the page size of the settings', async () => {
    stubApi({ 'GET config': CONFIG, 'GET log/0': pageOf([record('a user logged in')]) });
    render(LogViewer, { ondone: () => {} });

    const table = await screen.findByRole('table', { name: 'Log' });
    const cells = within(table)
      .getAllByRole('cell')
      .map((cell) => cell.textContent?.trim());
    expect(cells.slice(0, 6)).toEqual([
      '2026-11-20 03:15:00 UTC',
      'INFO',
      'Login',
      'a user logged in',
      '192.0.2.7',
      'zoe',
    ]);
    expect(screen.getByText('1 records. Page 1 of 1.')).toBeTruthy();
    const query = queries()[0];
    expect(query?.get('pageSize')).toBe('25');
    expect(query?.get('offset')).toBe('0');
    expect(query?.get('filter')).toBe('NONE');
    const choices = within(screen.getByRole('combobox', { name: 'File' }))
      .getAllByRole('option')
      .map((option) => option.textContent);
    expect(choices).toEqual(['All', '2026-11-20', '2026-11-19']);
  });

  it('filters by level, picks a file and turns the pages', async () => {
    const sent = stubApi({
      'GET config': CONFIG,
      'GET log/0': pageOf([record('one')], undefined, 30),
      'GET log/1': pageOf([record('older')]),
      'GET log/all': pageOf([record('everything')]),
    });
    render(LogViewer, { ondone: () => {} });
    await screen.findByText('one');

    await fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
    await vi.waitFor(() => expect(queries()[1]?.get('offset')).toBe('25'));

    await fireEvent.change(screen.getByRole('combobox', { name: 'Level' }), {
      target: { value: 'WARNING' },
    });
    await vi.waitFor(() => expect(queries()[2]?.get('filter')).toBe('WARNING'));
    expect(queries()[2]?.get('offset')).toBe('0');

    await fireEvent.change(screen.getByRole('combobox', { name: 'File' }), {
      target: { value: '1' },
    });
    expect(await screen.findByText('older')).toBeTruthy();
    await fireEvent.change(screen.getByRole('combobox', { name: 'File' }), {
      target: { value: 'all' },
    });
    expect(await screen.findByText('everything')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Purge Log' })).toHaveProperty('disabled', true);
    expect(sent.map((request) => request.path)).toContain('log/all');
  });

  it('opens the details of a record by its trace id', async () => {
    const full = {
      ...record('a login failed', 'WARNING', 'trace-1'),
      context: { attempts: 3 },
      request_id: 'request-9',
    };
    stubApi({
      'GET config': CONFIG,
      'GET log/0': pageOf([record('a login failed', 'WARNING', 'trace-1')]),
      'GET log/all/trace-1': { data: full },
    });
    render(LogViewer, { ondone: () => {} });

    const button = await screen.findByRole('button', { name: 'Details of a login failed' });
    await fireEvent.click(button);
    expect(await screen.findByText(/"attempts": 3/)).toBeTruthy();
    expect(screen.getByText('trace-1')).toBeTruthy();
    expect(screen.getByText('request-9')).toBeTruthy();
    expect(screen.getByText('crates/api/src/handlers/login.rs#L117')).toBeTruthy();
    expect(button.getAttribute('aria-expanded')).toBe('true');
  });

  it('purges the chosen file', async () => {
    let purged = false;
    const sent = stubApi({
      'GET config': CONFIG,
      'GET log/0': () =>
        purged ? { data: false, message: 'Results for log: ' } : pageOf([record('x')]),
      'DELETE log/0': () => {
        purged = true;
        return { message: 'Log purged' };
      },
    });
    const ondone = vi.fn();
    render(LogViewer, { ondone });

    await fireEvent.click(await screen.findByRole('button', { name: 'Purge Log' }));

    expect(await screen.findByText('There is no log file yet.')).toBeTruthy();
    expect(ondone).toHaveBeenCalledWith('The log file is purged.');
    expect(sent.filter((request) => request.method === 'DELETE')).toHaveLength(1);
  });

  it('loads the log again and again while the live update is on', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    stubApi({ 'GET config': CONFIG, 'GET log/0': pageOf([record('x')]) });
    render(LogViewer, { ondone: () => {} });
    await screen.findByText('x');
    expect(queries()).toHaveLength(1);

    const live = screen.getByRole('button', { name: 'Live update' });
    await fireEvent.click(live);
    expect(live.getAttribute('aria-pressed')).toBe('true');
    await vi.advanceTimersByTimeAsync(2500 * 2);
    expect(queries()).toHaveLength(3);

    await fireEvent.click(live);
    await vi.advanceTimersByTimeAsync(2500 * 2);
    expect(queries()).toHaveLength(3);
  });

  it('shows why the server refused', async () => {
    stubApi({
      'GET config': CONFIG,
      'GET log/0': { status: 401, message: 'Not Authorized' },
    });
    render(LogViewer, { ondone: () => {} });
    expect((await screen.findByRole('alert')).textContent).toContain('Not Authorized');
  });

  it('shows_the_log_settings_above_the_viewer_and_applies_a_saved_page_size_at_once', async () => {
    let stored: Record<string, unknown> = { ...CONFIG.data };
    const sent = stubApi({
      'GET config': () => ({ data: stored }),
      'PUT config': (request) => {
        stored = { ...stored, ...(request.body as Record<string, unknown>) };
        return { message: 'Config items updated' };
      },
      'GET log/0': pageOf([record('a user logged in')]),
      'GET launch': { data: { user: null } },
    });
    const ondone = vi.fn();
    render(LogViewer, { ondone });

    const table = await screen.findByRole('table', { name: 'Log' });
    const heading = screen.getByRole('heading', { name: 'Logs', level: 2 });
    expect(heading.compareDocumentPosition(table) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByLabelText('Log Page Size')).toHaveValue('25');

    await fireEvent.change(screen.getByLabelText('Log Page Size'), { target: { value: '50' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));
    expect(sent.filter((request) => request.method === 'PUT').map((r) => r.body)).toEqual([
      { logPageSize: '50' },
    ]);
    await vi.waitFor(() => expect(queries().at(-1)?.get('pageSize')).toBe('50'));
    // The settings load once for the form and the log, and once again after the save.
    expect(
      sent.filter((request) => request.method === 'GET' && request.path === 'config'),
    ).toHaveLength(2);
  });

  it('loads the settings once when it opens', async () => {
    const sent = stubApi({ 'GET config': CONFIG, 'GET log/0': pageOf([record('x')]) });
    render(LogViewer, { ondone: () => {} });

    await screen.findByRole('table', { name: 'Log' });
    expect(
      sent.filter((request) => request.method === 'GET' && request.path === 'config'),
    ).toHaveLength(1);
    expect(queries()[0]?.get('pageSize')).toBe('25');
  });

  it('shows the log with the default page size when the settings cannot be loaded', async () => {
    stubApi({
      'GET config': { status: 500, message: 'The database is busy' },
      'GET log/0': pageOf([record('x')]),
    });
    render(LogViewer, { ondone: () => {} });

    await screen.findByRole('table', { name: 'Log' });
    expect(queries()[0]?.get('pageSize')).toBe('50');
  });

  it('has an address of its own in the settings', () => {
    expect(parseRoute('#/settings/logs')).toEqual({ page: 'settings', section: 'logs' });
  });
});
