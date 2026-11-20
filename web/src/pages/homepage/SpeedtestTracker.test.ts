import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import SpeedtestTracker from './SpeedtestTracker.svelte';

const text = (element: HTMLElement) => element.textContent?.replace(/\s+/g, ' ') ?? '';

describe('SpeedtestTracker', () => {
  it('shows the last result with the average and the limits', async () => {
    stubApi({
      'GET homepage/speedtest/data': {
        data: {
          data: {
            current: { ping: '12.345', download: 250.12, upload: 40.26 },
            average: { ping: 13.5, download: 240, upload: 39 },
            maximum: { ping: 20, download: 260.5, upload: 41 },
            minimum: { ping: 9.5, download: 200.25, upload: 35 },
          },
          options: { title: 'Line', titleToggle: true },
        },
      },
    });
    render(SpeedtestTracker);

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
  });

  it('shows only the last result when the service sends no more, and no title when it is off', async () => {
    stubApi({
      'GET homepage/speedtest/data': {
        data: {
          data: { current: { ping: 5, download: 100, upload: 20 } },
          options: { title: 'Line', titleToggle: false },
        },
      },
    });
    render(SpeedtestTracker);

    const ping = await screen.findByRole('region', { name: 'Ping' });
    expect(text(ping)).toContain('5.0 ms (current)');
    expect(ping.textContent).not.toContain('average');
    expect(screen.queryByRole('heading', { name: 'Line' })).toBeNull();
  });

  it('says so when there is no result, and shows an error of the server', async () => {
    stubApi({
      'GET homepage/speedtest/data': {
        data: { data: { current: null }, options: { title: 'Line', titleToggle: true } },
      },
    });
    const { unmount } = render(SpeedtestTracker);
    expect(await screen.findByText('Speedtest Tracker has no result yet.')).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/speedtest/data': { status: 409, message: 'SpeedTest connection error' },
    });
    render(SpeedtestTracker);
    expect((await screen.findByRole('alert')).textContent).toContain('SpeedTest connection error');
  });
});
