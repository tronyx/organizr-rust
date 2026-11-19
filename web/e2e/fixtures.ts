// What every browser test gets: a page that fails the test when the browser reports a
// problem with the page itself.

import { test as base, expect } from '@playwright/test';

export const test = base.extend<{ problems: string[] }>({
  problems: [
    async ({ page }, use) => {
      const problems: string[] = [];
      page.on('pageerror', (error) => problems.push(`error in a script: ${error.message}`));
      page.on('console', (message) => {
        // A browser also reports an answer such as "401" of the API here. Tests look
        // at those answers themselves. Everything else is a problem: above all, a
        // script or a style that the Content-Security-Policy of the page stopped.
        const text = message.text();
        if (message.type() === 'error' && !text.startsWith('Failed to load resource')) {
          problems.push(`console: ${text}`);
        }
      });
      await use(problems);
      expect(problems, 'the browser reported problems with the page').toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };
