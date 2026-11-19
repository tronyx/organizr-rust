// One walk through the interface on a new install, from the first page on. The tests
// run in order and share the install.

import { expect, test } from './fixtures';
import { SETUP_CODE_VARIABLE } from './server';

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

/** What the set-up of the tests fills in. These values protect nothing. */
const ADMIN = {
  username: 'admin',
  email: 'admin@example.com',
  password: 'the password of the browser tests',
};

function setupCode(): string {
  const code = process.env[SETUP_CODE_VARIABLE];
  if (!code) {
    throw new Error('The start of the server did not give the set-up code.');
  }
  return code;
}

test('a new install shows the set-up wizard with a working style sheet', async ({ page }) => {
  await page.goto('./');

  await expect(page.getByRole('heading', { name: 'Set up Organizr' })).toBeVisible();
  await expect(page.getByText('SQLite')).toBeVisible();
  // The style sheet is in use: the page has the dark background of the theme.
  const background = await page.evaluate(
    () => getComputedStyle(document.documentElement).backgroundColor,
  );
  expect(background).not.toBe('rgba(0, 0, 0, 0)');
});

test('the wizard refuses a wrong set-up code and takes the right one', async ({ page }) => {
  await page.goto('./');
  await page.getByLabel('Set-up code').fill('AAAA-AAAA-AAAA');
  await page.getByLabel('User name').fill(ADMIN.username);
  await page.getByLabel('E-mail address').fill(ADMIN.email);
  await page.getByLabel('Password', { exact: true }).fill(ADMIN.password);
  await page.getByLabel('Password, once more').fill(ADMIN.password);
  await page.getByLabel('Registration password').fill('registration of the browser tests');

  await page.getByRole('button', { name: 'Set up' }).click();
  await expect(page.getByRole('alert')).toContainText('Setup code is missing or wrong');

  await page.getByLabel('Set-up code').fill(setupCode());
  await page.getByRole('button', { name: 'Set up' }).click();

  // The install is set up, and the wizard is gone.
  await expect(page.getByText('The server is running.')).toBeVisible();
  await page.reload();
  await expect(page.getByText('The server is running.')).toBeVisible();
});
