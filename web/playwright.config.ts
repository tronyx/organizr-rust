import { defineConfig, devices } from '@playwright/test';

import { ADDRESS } from './e2e/server';

// The tests run in a real browser against the real server binary, which hands out the
// built interface with its security headers. Build both first:
//
//   npm run build
//   cargo build -p organizr --features embedded-web
//   npm run e2e
export default defineConfig({
  testDir: 'e2e',
  testMatch: '*.spec.ts',
  globalSetup: './e2e/server.ts',
  // The tests share one server and one install, and each builds on the one before it.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: `http://${ADDRESS}/`,
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
