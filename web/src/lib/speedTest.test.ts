import { afterEach, describe, expect, it, vi } from 'vitest';

import { megabits, pingAndJitter, runSpeedTest, speedText } from './speedTest';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('megabits', () => {
  it('turns bytes and milliseconds into megabits per second', () => {
    // 1,250,000 bytes are ten megabits.
    expect(megabits(1_250_000, 1000)).toBe(10);
    expect(megabits(1_250_000, 500)).toBe(20);
    expect(megabits(100, 0)).toBe(0);
  });
});

describe('pingAndJitter', () => {
  it('gives the shortest time and the mean change', () => {
    expect(pingAndJitter([12, 10, 14])).toEqual({ ping: 10, jitter: 3 });
    expect(pingAndJitter([7])).toEqual({ ping: 7, jitter: 0 });
    expect(pingAndJitter([])).toEqual({ ping: 0, jitter: 0 });
  });
});

describe('speedText', () => {
  it('shows a fraction only for a slow line', () => {
    expect(speedText(8.44)).toBe('8.4');
    expect(speedText(84.4)).toBe('84');
    expect(speedText(null)).toBe('');
  });
});

describe('runSpeedTest', () => {
  function serve(refuse = false) {
    const asked: string[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init: RequestInit = {}) => {
        const path = new URL(url).pathname.replace(/^.*\/api\/v2\//, '');
        asked.push(`${init.method ?? 'GET'} ${path}`);
        if (refuse) {
          return new Response(
            JSON.stringify({ response: { message: 'SPEEDTEST-enabled module is not enabled' } }),
            { status: 409 },
          );
        }
        if (path === 'plugins/speedtest/ip') {
          return new Response(JSON.stringify({ response: { data: { ip: '192.0.2.7' } } }));
        }
        if (path === 'plugins/speedtest/garbage') {
          return new Response(new Uint8Array(250_000));
        }
        return new Response(null);
      }),
    );
    return asked;
  }

  it('measures the ping, the download and the upload, and names the address', async () => {
    const asked = serve();
    const steps: (string | null)[] = [];

    const result = await runSpeedTest({
      pings: 3,
      duration: 30,
      signal: new AbortController().signal,
      onprogress: (partial) => steps.push(partial.address),
    });

    expect(result.address).toBe('192.0.2.7');
    expect(result.ping).not.toBeNull();
    expect(result.jitter).not.toBeNull();
    expect(result.download).toBeGreaterThan(0);
    expect(result.upload).toBeGreaterThan(0);
    // One report after each of the four parts.
    expect(steps).toHaveLength(4);
    expect(asked[0]).toBe('GET plugins/speedtest/ip');
    expect(asked.filter((one) => one === 'GET plugins/speedtest/empty')).toHaveLength(3);
    expect(asked).toContain('GET plugins/speedtest/garbage');
    expect(asked).toContain('POST plugins/speedtest/empty');
  });

  it('fails with the reason of the server', async () => {
    serve(true);

    await expect(
      runSpeedTest({
        pings: 1,
        duration: 10,
        signal: new AbortController().signal,
        onprogress: () => {},
      }),
    ).rejects.toThrow('SPEEDTEST-enabled module is not enabled');
  });
});
