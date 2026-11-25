import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Netronome from './Netronome.svelte';

const text = (element: HTMLElement) => element.textContent?.replace(/\s+/g, ' ') ?? '';

const CURRENT = {
  ping: 12.345,
  download: 250.12,
  upload: 40.26,
  jitter: 1.25,
  server: 'A server',
  type: 'speedtest',
  time: '2026-11-03T10:00:00Z',
};

describe('Netronome', () => {
  it('shows the last test with the average and the limits of the time range', async () => {
    stubApi({
      'GET homepage/netronome/data': {
        data: {
          data: {
            current: CURRENT,
            average: { ping: 13.5, download: 240, upload: 39 },
            maximum: { ping: 20, download: 260.5, upload: 41 },
            minimum: { ping: 9.5, download: 200.25, upload: 35 },
            tests: 3,
          },
          options: { title: 'Line', titleToggle: true, timeRange: 'week' },
        },
      },
    });
    render(Netronome);

    const ping = await screen.findByRole('region', { name: 'Ping' });
    expect(text(ping)).toContain('12.3 ms (current)');
    expect(text(ping)).toContain('13.5 ms (average)');
    expect(text(ping)).toContain('20.0 ms (maximum)');
    expect(text(ping)).toContain('9.5 ms (minimum)');
    expect(text(screen.getByRole('region', { name: 'Download' }))).toContain(
      '250.1 Mbit/s (current)',
    );
    expect(text(screen.getByRole('region', { name: 'Upload' }))).toContain('40.3 Mbit/s (current)');
    expect(screen.getByRole('heading', { name: 'Line' })).toBeTruthy();

    const details = text(screen.getByTestId('netronome-details'));
    expect(details).toContain('A server · Speedtest.net · Jitter 1.3 ms · ');
    expect(details).toContain('2026');
    expect(details).toContain('· 3 tests in the last week');
  });

  it('shows only the last test when the answer has no more, and no title when it is off', async () => {
    stubApi({
      'GET homepage/netronome/data': {
        data: {
          data: {
            current: { ping: null, download: 100, upload: 20, jitter: null, type: 'a-new-kind' },
          },
          options: { title: 'Line', titleToggle: false, timeRange: '24h' },
        },
      },
    });
    render(Netronome);

    const ping = await screen.findByRole('region', { name: 'Ping' });
    // A test without a latency shows a dash.
    expect(text(ping)).toContain('– ms (current)');
    expect(ping.textContent).not.toContain('average');
    expect(screen.queryByRole('heading', { name: 'Line' })).toBeNull();
    const details = text(screen.getByTestId('netronome-details'));
    expect(details.trim()).toBe('a-new-kind');
  });

  it('says so when there is no test, and shows an error of the server', async () => {
    stubApi({
      'GET homepage/netronome/data': {
        data: {
          data: { current: null },
          options: { title: 'Line', titleToggle: true, timeRange: 'week' },
        },
      },
    });
    const { unmount } = render(Netronome);
    expect(await screen.findByText('Netronome has no speed test yet.')).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/netronome/data': { status: 409, message: 'Netronome connection error' },
    });
    render(Netronome);
    expect((await screen.findByRole('alert')).textContent).toContain('Netronome connection error');
  });
});
