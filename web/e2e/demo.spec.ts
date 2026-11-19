// One walk through the interface on a new install, from the first page on. The tests
// run in order and share the install.

import { expect, test } from './fixtures';

test.describe.configure({ mode: 'serial' });

test('the server hands out the page with a strict security policy', async ({ page }) => {
  const response = await page.goto('./');

  expect(response?.status()).toBe(200);
  const policy = response?.headers()['content-security-policy'] ?? '';
  expect(policy).toContain("script-src 'self'");
  expect(policy).toContain("style-src 'self'");
  expect(policy).not.toContain('unsafe');
  // The title of the page is the `title` setting, which the interface reads from the API.
  await expect(page).toHaveTitle('Organizr V2');
});

test('a new install says that it is not set up', async ({ page }) => {
  await page.goto('./');

  await expect(page.getByText('This install is not set up yet.')).toBeVisible();
  // The style sheet is in use: the page has the dark background of the theme.
  const background = await page.evaluate(
    () => getComputedStyle(document.documentElement).backgroundColor,
  );
  expect(background).not.toBe('rgba(0, 0, 0, 0)');
});
