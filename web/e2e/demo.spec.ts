// One walk through the interface on a new install, from the first page on. The tests
// run in order and share the install.

import type { Page } from '@playwright/test';

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

  // The install is set up, the wizard is gone, and the new Admin is logged in.
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
  await page.reload();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
});

/** Logs the Admin in. Each test has a browser of its own, without the cookies of the others. */
async function logIn(page: Page) {
  await page.goto('./#/login');
  await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in' }).click();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
}

test('the Admin logs out and sees the login page', async ({ page }) => {
  await logIn(page);

  await page.getByLabel('Menu of the user').click();
  await page.getByRole('button', { name: 'Log out' }).click();

  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByLabel('Menu of the user')).toHaveCount(0);
});

test('the login refuses a wrong password and takes the right one', async ({ page }) => {
  await page.goto('./#/login');
  await page.getByLabel('User name or e-mail address').fill(ADMIN.email);
  await page.getByLabel('Password').fill('not the password');
  await page.getByRole('button', { name: 'Log in' }).click();
  await expect(page.getByRole('alert')).toHaveText('User credentials incorrect');

  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in' }).click();

  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
  // The login lasts: the cookie of the session is sent again after a reload.
  await page.reload();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
});

test('the menu shows the tabs under their categories and opens them in frames', async ({
  page,
}) => {
  await logIn(page);
  // The tabs point at answers of the server itself, so the test needs no other site.
  const added = [
    await page.request.post('./api/v2/categories', {
      data: { category: 'Media', image: 'fontawesome::film', category_id: 7 },
    }),
    await page.request.post('./api/v2/tabs', {
      data: { name: 'Ping', url: '/api/v2/ping', image: 'fontawesome::heart', enabled: 1 },
    }),
    await page.request.post('./api/v2/tabs', {
      data: { name: 'Rox Page', url: '/api/v2/rox', image: '', enabled: 1, category_id: 7 },
    }),
    await page.request.post('./api/v2/tabs', {
      data: { name: 'Docs', url: 'https://example.com/docs', image: '', enabled: 1, type: 2 },
    }),
  ];
  expect(added.map((response) => response.status())).toEqual([200, 200, 200, 200]);
  await page.reload();

  const menu = page.getByRole('navigation', { name: 'Tabs' });
  await expect(menu.getByRole('link', { name: 'Ping' })).toBeVisible();
  await expect(menu.getByText('Media')).toBeVisible();
  const docs = menu.getByRole('link', { name: 'Docs' });
  await expect(docs).toHaveAttribute('href', 'https://example.com/docs');
  await expect(docs).toHaveAttribute('target', '_blank');

  // A tab opens in a frame, and the address names it as PHP Organizr does.
  await menu.getByRole('link', { name: 'Ping' }).click();
  await expect(page).toHaveURL(/#Ping$/);
  await expect(page.frameLocator('iframe[title="Ping"]').locator('body')).toContainText('pong');

  await menu.getByRole('link', { name: 'Rox Page' }).click();
  await expect(page).toHaveURL(/#Rox-Page$/);
  await expect(page.frameLocator('iframe[title="Rox Page"]').locator('body')).toContainText(
    'rox in socks!',
  );
  // The first frame stays loaded behind the second one.
  await expect(page.locator('iframe[title="Ping"]')).toBeHidden();
  await expect(page.locator('iframe')).toHaveCount(2);

  // The address leads back to the tab after a reload.
  await page.reload();
  await expect(page.locator('iframe[title="Rox Page"]')).toBeVisible();
  await expect(page.locator('iframe')).toHaveCount(1);
  await expect(menu.getByRole('link', { name: 'Rox Page' })).toHaveAttribute(
    'aria-current',
    'page',
  );
});

test('a guest sees no tab of the Admin and gets the login page', async ({ page }) => {
  await page.goto('./#Ping');

  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Tabs' }).getByRole('link')).toHaveCount(0);
  await expect(page.locator('iframe')).toHaveCount(0);
});
