import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { RunOptions, SpeedResult } from '../lib/speedTest';
import SpeedTest from './SpeedTest.svelte';

const run = vi.hoisted(() => vi.fn());

vi.mock('../lib/speedTest', async (original) => ({
  ...(await original<typeof import('../lib/speedTest')>()),
  runSpeedTest: run,
}));

const RESULT: SpeedResult = {
  ping: 12.4,
  jitter: 1.6,
  download: 84.4,
  upload: 8.44,
  address: '192.0.2.7',
};

afterEach(() => {
  run.mockReset();
});

describe('SpeedTest', () => {
  it('runs the test and shows what it found', async () => {
    run.mockImplementation(async (options: RunOptions) => {
      options.onprogress({ ...RESULT, upload: null });
      return RESULT;
    });
    render(SpeedTest);

    await fireEvent.click(screen.getByRole('button', { name: 'Start' }));

    expect(await screen.findByText('The test is done.')).toBeTruthy();
    const values = [...document.querySelectorAll('.stat-value')].map((value) => value.textContent);
    expect(values).toEqual(['84', '8.4', '12', '2']);
    expect(screen.getByText('Your address: 192.0.2.7')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Test again' })).toBeTruthy();
  });

  it('shows why the server refused the test', async () => {
    run.mockRejectedValue(new Error('SPEEDTEST-enabled module is not enabled'));
    render(SpeedTest);

    await fireEvent.click(screen.getByRole('button', { name: 'Start' }));

    expect((await screen.findByRole('alert')).textContent).toContain(
      'SPEEDTEST-enabled module is not enabled',
    );
    expect(screen.getByRole('button', { name: 'Start' })).toBeTruthy();
  });

  it('stops a run without an error', async () => {
    run.mockImplementation(
      (options: RunOptions) =>
        new Promise((_, reject) => {
          options.signal.addEventListener('abort', () => reject(new Error('aborted')));
        }),
    );
    render(SpeedTest);

    await fireEvent.click(screen.getByRole('button', { name: 'Start' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Stop' }));

    expect(await screen.findByRole('button', { name: 'Start' })).toBeTruthy();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
