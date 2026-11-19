// Starts the real server for the browser tests, on an empty data folder, and stops it
// afterwards.

import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/** Where the server of the browser tests listens. */
export const ADDRESS = '127.0.0.1:8171';

/** The name of the variable that holds the set-up code of the server for the tests. */
export const SETUP_CODE_VARIABLE = 'ORGANIZR_E2E_SETUP_CODE';

const SETUP_CODE = /set-up code ([0-9A-Z]{4}-[0-9A-Z]{4}-[0-9A-Z]{4})/;
const START_TIMEOUT_MS = 30_000;

function binaryPath(): string {
  const name = process.platform === 'win32' ? 'organizr.exe' : 'organizr';
  return process.env.ORGANIZR_BINARY ?? resolve(import.meta.dirname, '../../target/debug', name);
}

export default async function startServer(): Promise<() => Promise<void>> {
  const dataFolder = mkdtempSync(join(tmpdir(), 'organizr-e2e-'));
  const server = spawn(binaryPath(), ['serve'], {
    env: {
      ...process.env,
      ORGANIZR_LISTEN: ADDRESS,
      ORGANIZR_DATA_DIR: dataFolder,
      NO_COLOR: '1',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const exited = new Promise<void>((done) => {
    server.once('exit', () => done());
  });

  // A new install writes its set-up code to the log. The wizard asks for it.
  const setupCode = await new Promise<string>((found, failed) => {
    let log = '';
    const timer = setTimeout(() => {
      failed(new Error(`The server did not start in time. Its log:\n${log}`));
    }, START_TIMEOUT_MS);
    const read = (chunk: Buffer) => {
      log += chunk.toString();
      const code = SETUP_CODE.exec(log)?.[1];
      if (code && log.includes('Organizr is listening')) {
        clearTimeout(timer);
        found(code);
      }
    };
    server.stdout.on('data', read);
    server.stderr.on('data', read);
    server.once('error', (error) => {
      clearTimeout(timer);
      failed(new Error(`The server binary cannot be started: ${error.message}`));
    });
    server.once('exit', (code) => {
      clearTimeout(timer);
      failed(new Error(`The server stopped with the code ${code}. Its log:\n${log}`));
    });
  });
  process.env[SETUP_CODE_VARIABLE] = setupCode;

  return async () => {
    server.kill();
    await exited;
    rmSync(dataFolder, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  };
}
