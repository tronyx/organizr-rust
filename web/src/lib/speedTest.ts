// The speed test: measures the connection between this browser and Organizr.

import { apiUrl } from './api';

/** What a run found. A value is nothing until its part of the run is done. */
export interface SpeedResult {
  /** The shortest time of a request, in milliseconds. */
  ping: number | null;
  /** How much the time of a request changes, in milliseconds. */
  jitter: number | null;
  /** In megabits per second. */
  download: number | null;
  /** In megabits per second. */
  upload: number | null;
  /** The address that the server sees this browser at. */
  address: string | null;
}

/** How a run is done. A test gives short times. */
export interface RunOptions {
  /** How many requests measure the ping. */
  pings: number;
  /** How long the download and the upload each run, in milliseconds. */
  duration: number;
  /** Stops the run. */
  signal: AbortSignal;
  /** Called after each part with what is known so far. */
  onprogress: (result: SpeedResult) => void;
}

/** The megabits per second of a number of bytes in a number of milliseconds. */
export function megabits(bytes: number, milliseconds: number): number {
  if (milliseconds <= 0) {
    return 0;
  }
  return (bytes * 8) / (milliseconds / 1000) / 1_000_000;
}

/** The shortest time, and the mean change from one time to the next. */
export function pingAndJitter(times: number[]): { ping: number; jitter: number } {
  if (times.length === 0) {
    return { ping: 0, jitter: 0 };
  }
  let change = 0;
  for (let place = 1; place < times.length; place += 1) {
    change += Math.abs((times[place] ?? 0) - (times[place - 1] ?? 0));
  }
  return {
    ping: Math.min(...times),
    jitter: times.length > 1 ? change / (times.length - 1) : 0,
  };
}

/** A speed with as many digits as a person reads: "8.4", "84", "840". */
export function speedText(megabitsPerSecond: number | null): string {
  if (megabitsPerSecond === null) {
    return '';
  }
  return megabitsPerSecond < 10
    ? megabitsPerSecond.toFixed(1)
    : String(Math.round(megabitsPerSecond));
}

function address(path: string): string {
  // Each address is new, so no cache answers in the place of the server.
  const separator = path.includes('?') ? '&' : '?';
  return `${apiUrl(path)}${separator}r=${Math.random()}`;
}

async function passed(response: Response): Promise<Response> {
  if (!response.ok) {
    let message = `The server answered ${response.status}`;
    try {
      const body = (await response.json()) as { response?: { message?: string } };
      message = body.response?.message ?? message;
    } catch {
      // The answer has no message.
    }
    throw new Error(message);
  }
  return response;
}

async function measurePings(count: number, signal: AbortSignal): Promise<number[]> {
  const times: number[] = [];
  for (let request = 0; request < count; request += 1) {
    const start = performance.now();
    await passed(await fetch(address('plugins/speedtest/empty'), { signal, cache: 'no-store' }));
    times.push(performance.now() - start);
  }
  return times;
}

async function measureDownload(duration: number, signal: AbortSignal): Promise<number> {
  const start = performance.now();
  let bytes = 0;
  while (performance.now() - start < duration) {
    const response = await passed(
      await fetch(address('plugins/speedtest/garbage?ckSize=20'), { signal, cache: 'no-store' }),
    );
    const reader = response.body?.getReader();
    if (!reader) {
      bytes += (await response.arrayBuffer()).byteLength;
      continue;
    }
    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      bytes += value.byteLength;
      if (performance.now() - start >= duration) {
        await reader.cancel();
        break;
      }
    }
  }
  return megabits(bytes, performance.now() - start);
}

async function measureUpload(duration: number, signal: AbortSignal): Promise<number> {
  // One mebibyte of bytes that a proxy cannot compress.
  const block = new Uint8Array(1024 * 1024);
  for (let place = 0; place < block.length; place += 65536) {
    crypto.getRandomValues(block.subarray(place, place + 65536));
  }
  const start = performance.now();
  let bytes = 0;
  let blocks = 1;
  // One body is used for all requests of its size. A browser keeps each body that is
  // made for some time, and a fast line would fill the store of the browser with them.
  let body = new Blob([block]);
  while (performance.now() - start < duration) {
    const sent = performance.now();
    await passed(await fetch(address('plugins/speedtest/empty'), { method: 'POST', body, signal }));
    bytes += body.size;
    // A fast line gets larger requests, so the time of a request is mostly upload.
    if (performance.now() - sent < 500 && blocks < 32) {
      blocks *= 2;
      body = new Blob(Array.from({ length: blocks }, () => block));
    }
  }
  return megabits(bytes, performance.now() - start);
}

/** Runs the whole test. Fails with the reason when the server refuses a request. */
export async function runSpeedTest(options: RunOptions): Promise<SpeedResult> {
  const result: SpeedResult = {
    ping: null,
    jitter: null,
    download: null,
    upload: null,
    address: null,
  };
  const { signal, onprogress } = options;

  const told = await passed(await fetch(address('plugins/speedtest/ip'), { signal }));
  const body = (await told.json()) as { response?: { data?: { ip?: string } } };
  result.address = body.response?.data?.ip ?? null;
  onprogress({ ...result });

  Object.assign(result, pingAndJitter(await measurePings(options.pings, signal)));
  onprogress({ ...result });
  result.download = await measureDownload(options.duration, signal);
  onprogress({ ...result });
  result.upload = await measureUpload(options.duration, signal);
  onprogress({ ...result });
  return result;
}
