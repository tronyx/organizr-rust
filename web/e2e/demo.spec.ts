// One walk through the interface on a new install, from the first page on. The tests
// run in order and share the install.

import { createHmac } from 'node:crypto';

import type { Page } from '@playwright/test';

import { expect, test } from './fixtures';
import { ADDRESS, SETUP_CODE_VARIABLE } from './server';

test.describe.configure({ mode: 'serial' });

/** Switches a plugin on with its card under Plugins, Inactive, as in PHP. */
async function switchPluginOn(page: Page, name: string) {
  await page.goto('./#/settings/plugins/inactive');
  await page.getByRole('button', { name: `Switch ${name} on` }).click();
  await expect(page.getByRole('status')).toHaveText(`The plugin ${name} is switched on.`);
}

/** Saves a form of the settings, and waits until the server stored it and the form loaded again. */
async function saveSettings(page: Page) {
  const stored = page.waitForResponse(
    (answer) => answer.url().endsWith('/api/v2/config') && answer.request().method() === 'PUT',
  );
  const loaded = page.waitForResponse(
    (answer) =>
      /\/api\/v2\/(settings\/|plugins\/[a-z-]+\/settings)/.test(answer.url()) &&
      answer.request().method() === 'GET',
  );
  await page.getByRole('button', { name: 'Save' }).click();
  expect((await stored).ok()).toBe(true);
  await loaded;
  await expect(page.getByRole('button', { name: 'Save' })).toBeDisabled();
}

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

/** The address of the server of the tests. */
const ADDRESS_URL = `http://${ADDRESS}/`;

/** What the set-up of the tests fills in. These values protect nothing. */
const ADMIN = {
  username: 'admin',
  email: 'admin@example.com',
  password: 'the password of the browser tests',
};

/** The password that lets a person make an account. The set-up stores it. */
const REGISTRATION_PASSWORD = 'registration of the browser tests';

/** A PNG picture of one pixel, as base64. */
const ONE_PIXEL_PNG =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==';

/** The password that the Admin sets on the settings page later. */
const NEW_REGISTRATION_PASSWORD = 'a second registration password';

function setupCode(): string {
  const code = process.env[SETUP_CODE_VARIABLE];
  if (!code) {
    throw new Error('The start of the server did not give the set-up code.');
  }
  return code;
}

test('a new install shows the set-up wizard with a working style sheet', async ({ page }) => {
  await page.goto('./');

  await expect(page.getByRole('heading', { name: 'Organizr Setup Wizard' })).toBeVisible();
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
  await page.getByLabel('Registration password').fill(REGISTRATION_PASSWORD);

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
  await page.getByRole('banner').getByRole('button', { name: 'Logout' }).click();

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
  // The menu is a strip of pictures until the pointer is over it.
  await menu.hover();
  await expect(menu.getByRole('link', { name: 'Ping' })).toBeVisible();
  await expect(menu.getByText('Media')).toBeVisible();
  const docs = menu.getByRole('link', { name: 'Docs' });
  await expect(docs).toHaveAttribute('href', 'https://example.com/docs');
  await expect(docs).toHaveAttribute('target', '_blank');

  // A tab opens in a frame, and the address names it as PHP Organizr does.
  await menu.getByRole('link', { name: 'Ping' }).click();
  await expect(page).toHaveURL(/#Ping$/);
  await expect(page.frameLocator('iframe[title="Ping"]').locator('body')).toContainText('pong');

  // A category is closed until the person opens it.
  await menu.hover();
  await menu.getByText('Media').click();
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
  await menu.hover();
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

test('the Admin adds, changes, hides, moves and deletes a tab in the settings', async ({
  page,
}) => {
  await logIn(page);
  await page.getByLabel('Menu of the user').click();
  // The menu of tabs has a link with the same name: the tab "Settings".
  await page.getByRole('banner').getByRole('link', { name: 'Settings', exact: true }).click();
  await expect(page).toHaveURL(/#\/settings\/tabs$/);
  const menu = page.getByRole('navigation', { name: 'Tabs' });
  // The menu is a strip of pictures until the pointer is over it.
  await menu.hover();
  const status = page.getByRole('status');
  await expect(page.getByRole('row', { name: /Rox Page/ })).toContainText('Media');

  // Add.
  await page.getByRole('button', { name: 'Add a tab' }).click();
  const added = page.getByRole('dialog', { name: 'Add a tab' });
  await added.getByLabel('Name').fill('Status');
  await added.getByLabel('Address', { exact: true }).fill('/api/v2/status');
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The tab is added.');
  await expect(added).toHaveCount(0);
  await expect(menu.getByRole('link', { name: 'Status' })).toBeVisible();

  // A name that another tab has is refused inside the form.
  await page.getByRole('button', { name: 'Add a tab' }).click();
  await added.getByLabel('Name').fill('ping');
  await added.getByLabel('Address', { exact: true }).fill('/api/v2/ping');
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(added.getByRole('alert')).toHaveText('Tab name: ping is already taken');
  await added.getByRole('button', { name: 'Cancel' }).click();
  await expect(added).toHaveCount(0);

  // Change. The form shows the address as it is stored.
  await page
    .getByRole('row', { name: /Status/ })
    .getByRole('button', { name: /Change/ })
    .click();
  const changed = page.getByRole('dialog', { name: 'Change the tab Status' });
  await expect(changed.getByLabel('Address', { exact: true })).toHaveValue('/api/v2/status');
  await changed.getByLabel('Name').fill('Server Status');
  await changed.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The tab is changed.');
  await expect(menu.getByRole('link', { name: 'Server Status' })).toBeVisible();

  // Move it one place up.
  const names = page.getByRole('rowheader');
  await expect(names.last()).toContainText('Server Status');
  await page.getByLabel('Move Server Status up').click();
  await expect(status).toHaveText('The order is changed.');
  await expect(names.last()).not.toContainText('Server Status');

  // Take it out of the menu.
  await page.getByLabel('Show Server Status in the menu').click();
  await expect(menu.getByRole('link', { name: 'Server Status' })).toHaveCount(0);

  // Delete it.
  page.once('dialog', (dialog) => void dialog.accept());
  await page
    .getByRole('row', { name: /Server Status/ })
    .getByRole('button', { name: /Delete/ })
    .click();
  await expect(status).toHaveText('The tab is deleted.');
  await expect(page.getByRole('row', { name: /Server Status/ })).toHaveCount(0);
});

test('a tab shows a picture of Organizr or an icon in the menu', async ({ page }) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  const menu = page.getByRole('navigation', { name: 'Tabs' });
  // The menu is a strip of pictures until the pointer is over it.
  await menu.hover();
  const status = page.getByRole('status');
  const add = async (name: string, picture: string) => {
    await page.getByRole('button', { name: 'Add a tab' }).click();
    const added = page.getByRole('dialog', { name: 'Add a tab' });
    await added.getByLabel('Name').fill(name);
    await added.getByLabel('Address', { exact: true }).fill('/api/v2/ping');
    await added.getByLabel('Picture').fill(picture);
    await added.getByRole('button', { name: 'Save' }).click();
    await expect(status).toHaveText('The tab is added.');
    await expect(added).toHaveCount(0);
  };
  const remove = async (name: string) => {
    page.once('dialog', (dialog) => void dialog.accept());
    await page
      .getByRole('row', { name: new RegExp(name) })
      .getByRole('button', { name: /Delete/ })
      .click();
    await expect(menu.getByRole('link', { name })).toHaveCount(0);
  };

  // A picture that Organizr ships, at the path that PHP Organizr stores for it.
  await add('Plex Picture', 'plugins/images/tabs/plex.png');
  const picture = menu.getByRole('link', { name: 'Plex Picture' }).locator('img');
  await expect(picture).toHaveAttribute('src', 'plugins/images/tabs/plex.png');
  await expect
    .poll(() => picture.evaluate((image: HTMLImageElement) => image.naturalWidth))
    .toBeGreaterThan(0);

  // An icon, in the form that PHP Organizr stores for it. Its font comes from this server.
  await add('Cog Icon', 'fontawesome::cog');
  const icon = menu.getByRole('link', { name: 'Cog Icon' }).locator('i');
  await expect(icon).toHaveClass(/fa-cog/);
  await expect
    .poll(() => page.evaluate(() => document.fonts.check("16px 'organizr-fontawesome'")))
    .toBe(true);
  const drawn = await icon.evaluate((element) => ({
    font: getComputedStyle(element).fontFamily,
    glyph: getComputedStyle(element, '::before').content,
  }));
  expect(drawn.font).toContain('organizr-fontawesome');
  expect(drawn.glyph).not.toBe('none');

  await remove('Plex Picture');
  await remove('Cog Icon');

  // A picture that the Admin uploads in the chooser.
  await page.getByRole('button', { name: 'Add a tab' }).click();
  const added = page.getByRole('dialog', { name: 'Add a tab' });
  await added.getByLabel('Name').fill('Own Picture');
  await added.getByLabel('Address', { exact: true }).fill('/api/v2/ping');
  await added.getByRole('button', { name: 'Choose' }).click();
  const chooser = page.getByRole('dialog', { name: 'Choose a picture' });
  await expect(chooser.getByRole('button', { name: 'plex.png', exact: true })).toBeVisible();
  await chooser.getByRole('tab', { name: 'Upload' }).click();
  await chooser.getByLabel('Picture file').setInputFiles({
    name: 'own picture.png',
    mimeType: 'image/png',
    buffer: Buffer.from(ONE_PIXEL_PNG, 'base64'),
  });
  await chooser.getByRole('button', { name: 'Upload' }).click();
  await expect(chooser).toHaveCount(0);
  await expect(added.getByLabel('Picture', { exact: true })).toHaveValue(
    'data/userTabs/own picture.png',
  );
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The tab is added.');
  const own = menu.getByRole('link', { name: 'Own Picture' }).locator('img');
  await expect
    .poll(() => own.evaluate((image: HTMLImageElement) => image.naturalWidth))
    .toBeGreaterThan(0);

  // The chooser lists the uploaded picture, and deletes it.
  await page
    .getByRole('row', { name: /Own Picture/ })
    .getByRole('button', { name: /Change/ })
    .click();
  const changed = page.getByRole('dialog', { name: 'Change the tab Own Picture' });
  await changed.getByRole('button', { name: 'Choose' }).click();
  await chooser.getByLabel('Search').fill('own picture');
  page.once('dialog', (dialog) => void dialog.accept());
  await chooser.getByRole('button', { name: 'Delete own picture.png' }).click();
  await expect(chooser.getByText('No picture has this text in its name.')).toBeVisible();
  await chooser.getByRole('button', { name: 'Cancel' }).click();
  await changed.getByRole('button', { name: 'Cancel' }).click();
  await remove('Own Picture');
});

test('the Admin manages the categories, and one with tabs is not deleted', async ({ page }) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  await page.getByRole('tab', { name: 'Categories' }).click();
  await expect(page).toHaveURL(/#\/settings\/categories$/);
  const status = page.getByRole('status');
  await expect(page.getByRole('row', { name: /Media/ })).toBeVisible();

  await page.getByRole('button', { name: 'Add a category' }).click();
  const added = page.getByRole('dialog', { name: 'Add a category' });
  await added.getByLabel('Name').fill('Books');
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The category is added.');
  await expect(page.getByRole('row', { name: /Books/ })).toBeVisible();

  // The category "Media" holds the tab "Rox Page".
  page.once('dialog', (dialog) => void dialog.accept());
  await page
    .getByRole('row', { name: /Media/ })
    .getByRole('button', { name: /Delete/ })
    .click();
  await expect(page.getByRole('alert')).toHaveText('Category name: Media still has tabs');
  await expect(page.getByRole('row', { name: /Media/ })).toBeVisible();

  page.once('dialog', (dialog) => void dialog.accept());
  await page
    .getByRole('row', { name: /Books/ })
    .getByRole('button', { name: /Delete/ })
    .click();
  await expect(status).toHaveText('The category is deleted.');
  await expect(page.getByRole('row', { name: /Books/ })).toHaveCount(0);
});

test('the Admin manages the groups, and the group for new users is not deleted', async ({
  page,
}) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  await page.getByRole('tab', { name: 'User Management' }).click();
  await page.getByRole('tab', { name: 'Manage Groups' }).click();
  await expect(page).toHaveURL(/#\/settings\/groups$/);
  const status = page.getByRole('status');
  await expect(page.getByRole('row', { name: /Guest/ })).toBeVisible();

  await page.getByRole('button', { name: 'Add a group' }).click();
  const added = page.getByRole('dialog', { name: 'Add a group' });
  await added.getByLabel('Name').fill('Editors');
  await added.getByLabel('Picture').fill('https://example.com/editors.png');
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The group is added.');
  // The new group has no users yet.
  await expect(
    page
      .getByRole('row', { name: /Editors/ })
      .getByRole('cell')
      .nth(1),
  ).toHaveText('0');

  await page
    .getByRole('row', { name: /Editors/ })
    .getByRole('button', { name: /Change/ })
    .click();
  const changed = page.getByRole('dialog', { name: 'Change the group Editors' });
  await changed.getByLabel('Name').fill('User');
  await changed.getByRole('button', { name: 'Save' }).click();
  await expect(changed.getByRole('alert')).toHaveText('Group name: User is already taken');
  await changed.getByLabel('Name').fill('Writers');
  await changed.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The group is changed.');

  // New users join the group "User". The server keeps that group.
  const user = page.getByRole('row', { name: /^User/ });
  await expect(page.getByLabel('Put new users into User')).toHaveAttribute('aria-pressed', 'true');
  page.once('dialog', (dialog) => void dialog.accept());
  await user.getByRole('button', { name: /Delete/ }).click();
  await expect(page.getByRole('alert')).toHaveText(
    'Cannot delete group: User as it is the default group',
  );

  page.once('dialog', (dialog) => void dialog.accept());
  await page
    .getByRole('row', { name: /Writers/ })
    .getByRole('button', { name: /Delete/ })
    .click();
  await expect(status).toHaveText('The group is deleted.');
  await expect(page.getByRole('row', { name: /Writers/ })).toHaveCount(0);
});

test('the Admin manages the users, and cannot delete the own account', async ({ page }) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  await page.getByRole('tab', { name: 'User Management' }).click();
  await page.getByRole('tab', { name: 'Manage Users' }).click();
  await expect(page).toHaveURL(/#\/settings\/users$/);
  const status = page.getByRole('status');
  const own = page.getByRole('row', { name: new RegExp(ADMIN.username) });
  await expect(own.getByRole('button', { name: /Delete/ })).toBeDisabled();
  await expect(page.getByLabel(`Lock ${ADMIN.username}`)).toBeDisabled();

  await page.getByRole('button', { name: 'Add a user' }).click();
  const added = page.getByRole('dialog', { name: 'Add a user' });
  await added.getByLabel('User name').fill('abe');
  await added.getByLabel('E-mail address').fill('abe@example.com');
  await added.getByLabel('Password').fill('the password of abe');
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The user is added.');
  // A new user joins the group for new users.
  const abe = page.getByRole('row', { name: /abe@example\.com/ });
  await expect(abe.getByRole('cell', { name: 'User', exact: true })).toBeVisible();

  await abe.getByRole('button', { name: /Change/ }).click();
  const changed = page.getByRole('dialog', { name: 'Change the user abe' });
  await changed.getByLabel('Group').selectOption({ label: 'Co-Admin' });
  await changed.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The user is changed.');
  await expect(abe.getByRole('cell', { name: 'Co-Admin', exact: true })).toBeVisible();

  await page.getByLabel('Lock abe').check();
  await expect(status).toHaveText('The user is locked.');
  await expect(page.getByLabel('Lock abe')).toBeChecked();

  page.once('dialog', (dialog) => void dialog.accept());
  await abe.getByRole('button', { name: /Delete/ }).click();
  await expect(status).toHaveText('The user is deleted.');
  await expect(page.getByRole('row', { name: /abe@example\.com/ })).toHaveCount(0);
});

test('the Admin changes the name of the install and the registration password', async ({
  page,
}) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  // The name is in Customize, Appearance, Top Bar, as in PHP.
  await page.getByRole('tab', { name: 'Customize' }).click();
  await expect(page).toHaveURL(/#\/settings\/appearance$/);
  await page.getByRole('tab', { name: 'Top Bar' }).click();
  const name = page.getByLabel('Organizr Title');
  await expect(name).toHaveValue('Organizr V2');
  const save = page.getByRole('button', { name: 'Save' });
  await expect(save).toBeDisabled();
  await name.fill('Media at home');
  await save.click();
  await expect(page.getByRole('status')).toHaveText('The settings are saved.');
  // The bar and the title of the page follow at once.
  await expect(page).toHaveTitle('Media at home');
  await expect(page.getByRole('banner')).toContainText('Media at home');

  // The registration password is in System Settings, Main, Login.
  // As in PHP, System Settings opens About.
  await page.getByRole('tab', { name: 'System Settings' }).click();
  await expect(page).toHaveURL(/#\/settings\/about$/);
  await page
    .getByRole('tablist', { name: 'System Settings' })
    .getByRole('tab', { name: 'Main' })
    .click();
  await expect(page).toHaveURL(/#\/settings\/general$/);
  await page.getByRole('tab', { name: 'Login', exact: true }).click();
  await page.getByLabel('Registration Password', { exact: true }).fill(NEW_REGISTRATION_PASSWORD);
  await saveSettings(page);
  await expect(page.getByLabel('Registration Password', { exact: true })).toHaveValue('');

  // The server stored both. It shows the name, and for the password only that it has one.
  await page.reload();
  await page.getByRole('tab', { name: 'Login', exact: true }).click();
  await expect(page.getByLabel('Registration Password', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('Registration Password', { exact: true })).toHaveAttribute(
    'placeholder',
    'A value is stored',
  );
  await page.goto('./#/settings/appearance');
  await page.getByRole('tab', { name: 'Top Bar' }).click();
  await expect(page.getByLabel('Organizr Title')).toHaveValue('Media at home');
});

test('the Admin reads the log of the server on the settings page', async ({ page }) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  await page.getByRole('tab', { name: 'System Settings' }).click();
  await page
    .getByRole('tablist', { name: 'System Settings' })
    .getByRole('tab', { name: 'Logs' })
    .click();
  await expect(page).toHaveURL(/#\/settings\/logs$/);

  // The server writes its start to the log file of the day.
  const log = page.getByRole('table', { name: 'Log' });
  await expect(log).toContainText('Organizr is listening');

  await page.getByRole('combobox', { name: 'Level', exact: true }).selectOption('WARNING');
  await expect(log).not.toContainText('Organizr is listening');
  await page.getByRole('combobox', { name: 'Level', exact: true }).selectOption('NONE');
  await page.getByRole('button', { name: 'Details of Organizr is listening' }).click();
  await expect(log).toContainText('crates/server/src/serve.rs');
  // The start of the server is no request, so the record has no request id.
  await expect(log).toContainText(/Request ID\s*None/);
});

test('the Admin installs a theme, picks it, and sets a colour and custom CSS', async ({ page }) => {
  await logIn(page);
  const root = page.locator('html');
  const header = page.getByRole('banner');
  await expect(root).toHaveCSS('background-color', 'rgb(31, 31, 31)');

  await page.goto('./#/settings/themes');
  await page.getByRole('button', { name: 'Install Nord' }).click();
  await expect(page.getByRole('status')).toHaveText('The theme Nord is installed.');

  await page.getByRole('tab', { name: 'Appearance' }).click();
  await page.getByRole('tab', { name: 'Colors & Themes' }).click();
  const theme = page.getByLabel('Theme', { exact: true });
  await theme.selectOption('Nord');
  await page.getByLabel('Nav Bar Color', { exact: true }).fill('#112233');
  await page.getByRole('tab', { name: 'Custom CSS' }).click();
  await page
    .getByLabel('Custom CSS', { exact: true })
    .fill('.org-header { border-bottom: 3px solid rgb(1, 2, 3); }');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toHaveText('The settings are saved.');

  // The page has the colours of Nord, the colour of the setting, and the style sheet of
  // the Admin, all without a style that the security policy of the page would stop.
  await expect(root).toHaveAttribute('data-theme', 'org-nord');
  await expect(root).toHaveCSS('background-color', 'rgb(46, 52, 64)');
  await expect(header).toHaveCSS('background-color', 'rgb(17, 34, 51)');
  await expect(header).toHaveCSS('border-bottom-color', 'rgb(1, 2, 3)');
  await page.reload();
  await expect(header).toHaveCSS('border-bottom-color', 'rgb(1, 2, 3)');
  await expect(root).toHaveCSS('background-color', 'rgb(46, 52, 64)');

  // Back to the look of a new install, for the tests after this one.
  await page.getByRole('tab', { name: 'Colors & Themes' }).click();
  await page.getByLabel('Theme', { exact: true }).selectOption('Organizr');
  await page.getByLabel('Nav Bar Color', { exact: true }).fill('');
  await page.getByRole('tab', { name: 'Custom CSS' }).click();
  await page.getByLabel('Custom CSS', { exact: true }).fill('');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toHaveText('The settings are saved.');
  await expect(root).toHaveAttribute('data-theme', 'organizr');
  await expect(root).toHaveCSS('background-color', 'rgb(31, 31, 31)');
  await expect(header).not.toHaveCSS('border-bottom-color', 'rgb(1, 2, 3)');
});

test('the Admin adds a custom script, which every page runs while the switch is on', async ({
  page,
}) => {
  await logIn(page);
  await page.goto('./#/settings/appearance');
  await page.getByRole('tab', { name: 'Custom Javascript' }).click();
  await expect(page.getByText(/The scripts run now/)).toBeVisible();
  await page
    .getByLabel('Custom Javascript', { exact: true })
    .fill("document.documentElement.setAttribute('data-custom-script', 'ran');");
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toHaveText('The settings are saved.');

  // The page loads the script from its own site, under `script-src 'self'`.
  const root = page.locator('html');
  await expect(root).toHaveAttribute('data-custom-script', 'ran');
  await page.reload();
  await expect(root).toHaveAttribute('data-custom-script', 'ran');
  // A guest on the login page runs it too, as in PHP.
  await page.getByLabel('Menu of the user').click();
  await page.getByRole('banner').getByRole('button', { name: 'Logout' }).click();
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await page.reload();
  await expect(root).toHaveAttribute('data-custom-script', 'ran');

  // Without a script there is no file, and the page loads none.
  await logIn(page);
  await page.goto('./#/settings/appearance');
  await page.getByRole('tab', { name: 'Custom Javascript' }).click();
  await page.getByLabel('Custom Javascript', { exact: true }).fill('');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toHaveText('The settings are saved.');
  await page.reload();
  await expect(root).not.toHaveAttribute('data-custom-script', 'ran');
  await expect(page.locator('script#org-custom-js')).toHaveCount(0);
  expect((await page.request.get('./api/v2/appearance/custom.js')).status()).toBe(404);
});

test('a guest cannot open the settings', async ({ page }) => {
  await page.goto('./#/settings/tabs');

  // A guest is asked to log in, and the API refuses the data of the editor.
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  const refused = await page.request.get('./api/v2/tabs');
  expect(refused.status()).toBe(401);
});

test('a person with the registration password makes an account and sees the tabs of a user', async ({
  page,
}) => {
  await page.goto('./#/login');
  await page.getByRole('link', { name: 'Make an account' }).click();
  await expect(page).toHaveURL(/#\/register$/);
  await expect(page.getByRole('heading', { name: 'Make an account' })).toBeVisible();
  await page.getByLabel('User name', { exact: true }).fill('zoe');
  await page.getByLabel('E-mail address').fill('zoe@example.com');
  await page.getByLabel('Password', { exact: true }).fill('the password of zoe');
  await page.getByLabel('Password, once more').fill('the password of zoe');

  // The password of the set-up was replaced on the settings page.
  await page.getByLabel('Registration password').fill(REGISTRATION_PASSWORD);
  await page.getByRole('button', { name: 'Make the account' }).click();
  await expect(page.getByRole('alert')).toHaveText('Registration Password was incorrect');

  await page.getByLabel('Registration password').fill(NEW_REGISTRATION_PASSWORD);
  await page.getByRole('button', { name: 'Make the account' }).click();

  // The new user is logged in, in the default group, and sees the tabs of that group.
  await expect(page.getByLabel('Menu of the user')).toHaveText('zoe');
  const menu = page.getByRole('navigation', { name: 'Tabs' });
  // The menu is a strip of pictures until the pointer is over it.
  await menu.hover();
  await expect(menu.getByRole('link', { name: 'Ping' })).toBeVisible();
  // The tab "Settings" is for Admins and Co-Admins, and so are the settings.
  await expect(menu.getByRole('link', { name: 'Settings' })).toHaveCount(0);
  await page.getByLabel('Menu of the user').click();
  await expect(
    page.getByRole('banner').getByRole('link', { name: 'Settings', exact: true }),
  ).toHaveCount(0);
  await page.goto('./#/settings/tabs');
  await expect(page.getByText('Only an Admin or a Co-Admin can open the settings.')).toBeVisible();
  const refused = await page.request.get('./api/v2/tabs');
  expect(refused.status()).toBe(401);
});

test('the menu shows whether the application behind a tab answers', async ({ page }) => {
  await logIn(page);
  // The first address is the server of the tests itself. Nothing listens on the second.
  const added = [
    await page.request.post('./api/v2/tabs', {
      data: {
        name: 'Answers',
        url: '/api/v2/ping',
        image: '',
        enabled: 1,
        ping: 1,
        ping_url: ADDRESS,
      },
    }),
    await page.request.post('./api/v2/tabs', {
      data: {
        name: 'Silent',
        url: '/api/v2/rox',
        image: '',
        enabled: 1,
        ping: 1,
        ping_url: '127.0.0.1:1',
      },
    }),
  ];
  expect(added.map((response) => response.status())).toEqual([200, 200]);
  await page.reload();

  const menu = page.getByRole('navigation', { name: 'Tabs' });
  // The menu is a strip of pictures until the pointer is over it.
  await menu.hover();
  await expect(menu.getByRole('link', { name: 'Answers' }).getByRole('img')).toHaveAccessibleName(
    'answers',
  );
  await expect(menu.getByRole('link', { name: 'Silent' }).getByRole('img')).toHaveAccessibleName(
    'does not answer',
  );
  // A tab without the check has no dot.
  await expect(menu.getByRole('link', { name: 'Ping' }).getByRole('img')).toHaveCount(0);

  // The form of the tab shows the stored check.
  await page.goto('./#/settings/tabs');
  await page
    .getByRole('row', { name: /Silent/ })
    .getByRole('button', { name: /Change/ })
    .click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByLabel('Checked whether it answers')).toBeChecked();
  await expect(dialog.getByLabel('Address to check')).toHaveValue('127.0.0.1:1');
});

/** The code of an authenticator app for a secret: RFC 6238 with SHA-1 and six digits. */
function authenticatorCode(secret: string): string {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = '';
  for (const character of secret.replace(/=+$/, '').toUpperCase()) {
    bits += alphabet.indexOf(character).toString(2).padStart(5, '0');
  }
  const key = Buffer.from((bits.match(/.{8}/g) ?? []).map((byte) => Number.parseInt(byte, 2)));
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const digest = createHmac('sha1', key).update(counter).digest();
  const offset = (digest.at(-1) ?? 0) & 0x0f;
  const number = digest.readUInt32BE(offset) & 0x7fffffff;
  return String(number % 1_000_000).padStart(6, '0');
}

test('a user sets a second factor up, and the login asks for its code', async ({ page }) => {
  await logIn(page);
  await page.getByLabel('Menu of the user').click();
  await page.getByRole('banner').getByRole('link', { name: 'Account Settings' }).click();
  await expect(page.getByRole('heading', { name: 'Second factor' })).toBeVisible();
  await expect(page.getByText('Your account has no second factor.')).toBeVisible();

  await page.getByRole('button', { name: 'Set a second factor up' }).click();
  // The QR code is a picture inside the page. Nothing is loaded from another site.
  const picture = page.getByAltText('QR code for the authenticator app');
  await expect(picture).toHaveAttribute('src', /^data:image\/png;base64,/);
  await expect
    .poll(() => picture.evaluate((image) => (image as HTMLImageElement).naturalWidth))
    .toBeGreaterThan(0);
  const secret = (await page.locator('code').innerText()).trim();
  expect(secret).toMatch(/^[A-Z2-7]{32}$/);

  await page.getByLabel('Enter the code that the app shows.').fill(authenticatorCode(secret));
  await page.getByRole('button', { name: 'Switch the second factor on' }).click();
  await expect(page.getByRole('status')).toHaveText(/The second factor is on\./);
  await expect(page.getByText('Your account has a second factor.')).toBeVisible();

  // The next login needs the code.
  await page.getByLabel('Menu of the user').click();
  await page.getByRole('banner').getByRole('button', { name: 'Logout' }).click();
  // The logout is done when the login page shows.
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByLabel('Menu of the user')).toHaveCount(0);
  await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in' }).click();
  const code = page.getByLabel('Code from your authenticator app');
  await expect(code).toBeVisible();
  await expect(page.getByLabel('Menu of the user')).toHaveCount(0);
  await code.fill(authenticatorCode(secret));
  await page.getByRole('button', { name: 'Log in' }).click();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);

  // Off again, so that the tests after this one log in with the password alone.
  await page.goto('./#/account');
  page.once('dialog', (dialog) => void dialog.accept());
  await page.getByRole('button', { name: 'Switch the second factor off' }).click();
  await expect(page.getByRole('status')).toHaveText('The second factor is off.');
  await expect(page.getByText('Your account has no second factor.')).toBeVisible();
});

test('a new password by e-mail is offered only when the install sends e-mails', async ({
  page,
}) => {
  // The mail plugin is off: the login page has no link, and the server says why.
  await page.goto('./#/login');
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Forgot your password?' })).toHaveCount(0);
  await page.goto('./#/recover');
  await expect(page.getByRole('heading', { name: 'Get a new password' })).toBeVisible();
  await page.getByLabel('E-mail address', { exact: true }).fill(ADMIN.email);
  await page.getByRole('button', { name: 'Send the e-mail' }).click();
  await expect(page.getByRole('alert')).toHaveText('Email functionality is not enabled');

  // The Admin switches the plugin PHP Mailer on, as in PHP, and sets it up in its
  // dialog. No mail server listens at the address of the test.
  await logIn(page);
  await page.goto('./#/settings/plugins/inactive');
  await page.getByRole('button', { name: 'Switch PHP Mailer on' }).click();
  await expect(page.getByRole('status')).toHaveText('The plugin PHP Mailer is switched on.');
  await page.goto('./#/settings/plugins');
  await page.getByRole('button', { name: 'Settings of PHP Mailer' }).click();
  const mailer = page.getByRole('dialog', { name: 'PHP Mailer Settings' });
  await mailer.getByRole('tab', { name: 'Host' }).click();
  await mailer.getByLabel('SMTP Host').fill('127.0.0.1');
  await mailer.getByLabel('SMTP Port').fill('1');
  await mailer.getByRole('tab', { name: 'Authentication', exact: true }).click();
  await mailer.getByLabel('Authentication Type').selectOption('n/a');
  await mailer.getByLabel('Authentication', { exact: true }).uncheck();
  await saveSettings(page);
  await mailer.getByRole('tab', { name: 'Test & Options' }).click();
  await mailer.getByRole('button', { name: 'Send' }).click();
  await expect(mailer.getByRole('alert')).toBeVisible();
  await page.keyboard.press('Escape');

  await page.getByLabel('Menu of the user').click();
  await page.getByRole('banner').getByRole('button', { name: 'Logout' }).click();
  // The logout is done when the login page shows.
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByLabel('Menu of the user')).toHaveCount(0);
  await page.getByRole('link', { name: 'Forgot your password?' }).click();
  await expect(page).toHaveURL(/#\/recover$/);
  // The login page has a field with a similar label, so the test waits for this page.
  await expect(page.getByRole('heading', { name: 'Get a new password' })).toBeVisible();
  await page.getByLabel('E-mail address', { exact: true }).fill('nobody@example.com');
  await page.getByRole('button', { name: 'Send the e-mail' }).click();
  await expect(page.getByRole('status')).toContainText('If an account has this address');
  await page.getByRole('link', { name: 'Back to the login' }).click();
  await expect(page).toHaveURL(/#\/login$/);
});

test('after a login the browser goes back to the page that a proxy held back', async ({
  page,
  baseURL,
}) => {
  // A reverse proxy sends a person who is not logged in here, with the address that the
  // person wanted. Here that address is a page of this server itself.
  const wanted = `${baseURL}api/v2/ping`;
  await page.goto(`./?return=${wanted}#/login`);
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in' }).click();

  await expect(page).toHaveURL(wanted);
  await expect(page.locator('body')).toContainText('pong');
});

test('an address of another site in the link is not followed', async ({ page, context }) => {
  await context.clearCookies();
  await page.goto('./?return=https://evil.example.net/#/login');
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in' }).click();

  // The person is logged in and stays in Organizr.
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
  expect(new URL(page.url()).host).toBe(new URL(ADDRESS_URL).host);
});

test('the homepage shows custom HTML of the Admin in a frame that keeps it apart', async ({
  page,
}) => {
  await logIn(page);
  await page.goto('./#/settings/tabs');
  // The set-up makes the tab "Homepage" and leaves it out of the menu.
  await page.getByLabel('Show Homepage in the menu').check();
  await expect(page.getByRole('status')).toHaveText('The tab is changed.');
  await page.getByRole('tab', { name: 'Homepage Items' }).click();
  await expect(page).toHaveURL(/#\/settings\/homepage$/);

  await page.getByRole('button', { name: 'Settings of CustomHTML' }).click();
  const item = page.getByRole('dialog', { name: 'CustomHTML' });
  await item.getByRole('tab', { name: 'Custom HTML 01' }).click();
  await item.getByLabel('Enable', { exact: true }).check();
  // The script tries to read the cookies of Organizr, which hold the session.
  await item
    .getByLabel('Custom HTML Code')
    .fill(
      '<p>Hello from custom HTML</p><p id="cookies"></p>' +
        '<div style="height: 400px"></div>' +
        '<a href="#/settings/groups">Open the groups</a>' +
        '<script>var seen; try { seen = "cookies: " + document.cookie; } catch (e) { seen = "no cookies for this frame"; }' +
        ' document.getElementById("cookies").textContent = seen;</script>',
    );
  await saveSettings(page);

  await page.goto('./#Homepage');
  await page.reload();
  const frame = page.frameLocator('iframe[title="Custom HTML 01"]');
  await expect(frame.getByText('Hello from custom HTML')).toBeVisible();
  // The script of the item runs, and the browser keeps the cookies from it.
  await expect(frame.locator('#cookies')).toHaveText('no cookies for this frame');
  // The frame is as high as its content.
  await expect
    .poll(async () => (await page.locator('iframe[title="Custom HTML 01"]').boundingBox())?.height)
    .toBeGreaterThan(400);

  // A link of the item changes the page of Organizr.
  await frame.getByRole('link', { name: 'Open the groups' }).click();
  await expect(page).toHaveURL(/#\/settings\/groups$/);
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
});

test('the chat plugin shows a new message at once, as text', async ({ page }) => {
  await logIn(page);
  await switchPluginOn(page, 'Chat');

  await page.getByLabel('Menu of the user').click();
  await page.getByRole('link', { name: 'Chat' }).click();
  await expect(page).toHaveURL(/#\/chat$/);
  await expect(page.getByText('No message yet.')).toBeVisible();
  const field = page.getByRole('textbox', { name: 'Message' });
  await field.fill('<b>Hello</b> & welcome');
  await field.press('Enter');
  // The message comes back through the stream of the server, as text and not as HTML.
  const messages = page.getByRole('list', { name: 'Messages' });
  await expect(messages).toContainText('<b>Hello</b> & welcome');
  await expect(messages.locator('b')).toHaveCount(0);
  await expect(field).toHaveValue('');
});

test('the speed test plugin measures the connection to the server', async ({ page }) => {
  // The run itself takes about twenty seconds.
  test.setTimeout(90_000);
  await logIn(page);
  await switchPluginOn(page, 'SpeedTest');

  // The menu of the user has the entry now.
  await page.getByLabel('Menu of the user').click();
  await page.getByRole('link', { name: 'Speed test' }).click();
  await expect(page).toHaveURL(/#\/speedtest$/);
  await expect(page.getByRole('heading', { name: 'Speed test' })).toBeVisible();

  await page.getByRole('button', { name: 'Start' }).click();
  await expect(page.getByText('The test runs.')).toBeVisible();
  await expect(page.getByText('The test is done.')).toBeVisible({ timeout: 60_000 });
  // Each of the four values is a number.
  const values = await page.locator('.stat-value').allTextContents();
  expect(values).toHaveLength(4);
  for (const value of values) {
    expect(value).toMatch(/^\d+(\.\d)?$/);
  }
  await expect(page.getByText(/^Your address: /)).toBeVisible();
});

test('the Admin adds a bookmark, and the page of the bookmarks shows it as a tile', async ({
  page,
}) => {
  await logIn(page);
  await switchPluginOn(page, 'Bookmark');
  await page.goto('./#/settings/plugins');
  await page.getByRole('button', { name: 'Settings of Bookmark' }).click();
  await page
    .getByRole('dialog', { name: 'Bookmark Settings' })
    .getByRole('tab', { name: 'Bookmarks' })
    .click();

  // The editor shows once the plugin is on.
  await page.getByLabel('Name of a new category').fill('Reading');
  await page.getByRole('button', { name: 'Add a category' }).click();
  await expect(page.getByRole('status')).toHaveText('The category is added.');
  await page.getByRole('button', { name: 'Add a bookmark' }).click();
  const dialog = page.getByRole('dialog', { name: 'Add a bookmark' });
  await dialog.getByLabel('Name', { exact: true }).fill('Example');
  await dialog.getByLabel('Address').fill('https://example.com/');
  await dialog.getByLabel('Colour of the tile').fill('#804020');
  await dialog.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toHaveText('The bookmark is added.');
  await expect(page.getByRole('table', { name: 'Bookmarks' })).toContainText('Example');

  await page.getByRole('button', { name: 'Make the tab Bookmarks' }).click();
  await expect(page.getByRole('status')).toHaveText('The menu has the tab Bookmarks.');

  await page.goto('./#Bookmarks');
  await page.reload();
  const tile = page.getByRole('region', { name: 'Reading' }).getByRole('link', { name: 'Example' });
  await expect(tile).toBeVisible();
  await expect(tile).toHaveAttribute('href', 'https://example.com/');
  // The colours of the tile reach the page, also with the strict security policy.
  await expect(tile).toHaveCSS('color', 'rgb(255, 255, 255)');
  const background = await tile.evaluate((element) => getComputedStyle(element).backgroundImage);
  expect(background).toContain('linear-gradient');
});

// This test stays the last one: after it, the login page has a second button whose name
// starts with "Log in".
test('the Admin makes an invite, and a guest finds the page for its code', async ({ page }) => {
  await logIn(page);
  await switchPluginOn(page, 'Invites');

  await page.getByLabel('Menu of the user').click();
  await page.getByRole('link', { name: 'Invites' }).click();
  await expect(page).toHaveURL(/#\/invites$/);
  await expect(page.getByText('There are no invites yet.')).toBeVisible();
  await page.getByLabel('Name of the person').fill('Sam');
  await page.getByLabel('E-mail address').fill('sam@example.com');
  await page.getByRole('button', { name: 'Make an invite' }).click();
  await expect(page.getByRole('status')).toHaveText(/^Invite Code: [A-Z0-9]{6} has been created$/);
  const table = page.getByRole('table', { name: 'Invites' });
  await expect(table).toContainText('Not used');
  const code = (await table.locator('tbody td').first().textContent()) ?? '';
  expect(code).toMatch(/^[A-Z0-9]{6}$/);

  // A guest gets the page from the login page, and from the link of the e-mail.
  await page.context().clearCookies();
  await page.goto('./#/login');
  await page.reload();
  await page.getByRole('link', { name: 'Use an invite code' }).click();
  await expect(page.getByRole('heading', { name: 'Use an invite code' })).toBeVisible();
  await page.goto(`./?invite=${code}`);
  await expect(page.getByLabel('Invite code')).toHaveValue(code);
  await page.getByLabel('Plex user name or e-mail address').fill('sam@example.com');
  await page.getByRole('button', { name: 'Use the code' }).click();
  // This install has no Plex server, so the code stays good.
  await expect(page.getByRole('alert')).toHaveText('Plex Token/ID not set');

  await page.getByLabel('Invite code').fill('WRONG1');
  await page.getByRole('button', { name: 'Use the code' }).click();
  await expect(page.getByRole('alert')).toHaveText('Code is invalid');
});

test('the Admin makes a backup, sees it in the list and deletes it', async ({ page }) => {
  await logIn(page);
  await page.goto('./#/settings/backups');
  await expect(page.getByText('There are no backups yet.')).toBeVisible();
  await page.getByRole('button', { name: 'Make a backup now' }).click();
  await expect(page.getByRole('status')).toHaveText('The backup is made.');
  const table = page.getByRole('table', { name: 'Backups' });
  await expect(table).toContainText(/backup\[\d{4}-\d\d-\d\d_\d\d-\d\d - \w\w\]\[[\d.]+\]\.zip/);

  // The link sends the zip file.
  const link = table.getByRole('link', { name: /^Save backup/ });
  const answer = await page.request.get((await link.getAttribute('href')) ?? '');
  expect(answer.status()).toBe(200);
  expect(answer.headers()['content-type']).toBe('application/zip');
  expect((await answer.body()).subarray(0, 2).toString()).toBe('PK');

  await table.getByRole('button', { name: /^Delete backup/ }).click();
  await expect(page.getByRole('status')).toHaveText('The backup is deleted.');
  await expect(page.getByText('There are no backups yet.')).toBeVisible();
});

test('a user locks the screen and opens it again with the password', async ({ page }) => {
  await logIn(page);
  await page.getByLabel('Menu of the user').click();
  await page.getByRole('button', { name: 'Lock Screen' }).click();
  await expect(page.getByRole('heading', { name: 'The screen is locked' })).toBeVisible();
  // The lock holds after the page is loaded again.
  await page.reload();
  await expect(page.getByRole('heading', { name: 'The screen is locked' })).toBeVisible();
  await expect(page.getByLabel('Menu of the user')).toHaveCount(0);

  await page.getByLabel('Password').fill('not the password');
  await page.getByRole('button', { name: 'Unlock' }).click();
  await expect(page.getByRole('alert')).toHaveText('Password Incorrect');

  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Unlock' }).click();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
});

test('the Admin switches the button "Log in with Plex" on, and the login page shows it', async ({
  page,
  context,
}) => {
  await context.clearCookies();
  await page.goto('./#/login');
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Log in with Plex' })).toHaveCount(0);

  await logIn(page);
  await page.goto('./#/settings/tabs');
  await page.getByRole('tab', { name: 'System Settings' }).click();
  await page
    .getByRole('tablist', { name: 'System Settings' })
    .getByRole('tab', { name: 'SSO' })
    .click();
  await expect(page).toHaveURL(/#\/settings\/login$/);

  // No Plex token is stored, so Get Plex Machine has no account to list the servers of.
  await page.getByRole('tab', { name: 'Plex', exact: true }).click();
  await page.getByRole('button', { name: 'Retrieve' }).nth(1).click();
  await expect(page.getByRole('alert')).toHaveText('Plex Token cannot be empty');

  // PHP shows "Enable Plex oAuth" under Main, Authentication, for the back end Plex.
  await page
    .getByRole('tablist', { name: 'System Settings' })
    .getByRole('tab', { name: 'Main' })
    .click();
  await page.getByRole('tab', { name: 'Authentication' }).click();
  await page.getByLabel('Authentication Type').selectOption('both');
  await page.getByLabel('Authentication Backend').selectOption('plex');
  await page.getByLabel('Enable Plex oAuth').check();
  await saveSettings(page);
  // Organizr checks the passwords alone again; the button stays on.
  await page.getByLabel('Authentication Type').selectOption('internal');
  await saveSettings(page);

  await context.clearCookies();
  await page.goto('./#/login');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Log in with Plex' })).toBeVisible();
});
