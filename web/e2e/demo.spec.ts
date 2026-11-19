// One walk through the interface on a new install, from the first page on. The tests
// run in order and share the install.

import type { Page } from '@playwright/test';

import { expect, test } from './fixtures';
import { ADDRESS, SETUP_CODE_VARIABLE } from './server';

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

test('the Admin adds, changes, hides, moves and deletes a tab in the settings', async ({
  page,
}) => {
  await logIn(page);
  await page.getByLabel('Menu of the user').click();
  // The menu of tabs has a link with the same name: the tab "Settings".
  await page.getByRole('banner').getByRole('link', { name: 'Settings' }).click();
  await expect(page).toHaveURL(/#\/settings\/tabs$/);
  const menu = page.getByRole('navigation', { name: 'Tabs' });
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
  await page.getByRole('tab', { name: 'Groups' }).click();
  await expect(page).toHaveURL(/#\/settings\/groups$/);
  const status = page.getByRole('status');
  await expect(page.getByRole('row', { name: /Guest/ })).toBeVisible();

  await page.getByRole('button', { name: 'Add a group' }).click();
  const added = page.getByRole('dialog', { name: 'Add a group' });
  await added.getByLabel('Name').fill('Editors');
  await added.getByLabel('Picture').fill('https://example.com/editors.png');
  await added.getByRole('button', { name: 'Save' }).click();
  await expect(status).toHaveText('The group is added.');
  // The server gives the new group the level after "User", which has the level 4.
  await expect(
    page
      .getByRole('row', { name: /Editors/ })
      .getByRole('cell')
      .first(),
  ).toHaveText('5');

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
  await expect(page.getByLabel('Put new users into User')).toBeChecked();
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
  await page.getByRole('tab', { name: 'Users' }).click();
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
  await page.getByRole('tab', { name: 'General' }).click();
  await expect(page).toHaveURL(/#\/settings\/general$/);
  const name = page.getByLabel('Name of this install');
  await expect(name).toHaveValue('Organizr V2');
  const save = page.getByRole('button', { name: 'Save' });
  await expect(save).toBeDisabled();

  await name.fill('Media at home');
  await page.getByLabel('Registration password').fill(NEW_REGISTRATION_PASSWORD);
  await save.click();
  await expect(page.getByRole('status')).toHaveText('The settings are saved.');
  // The bar and the title of the page follow at once.
  await expect(page).toHaveTitle('Media at home');
  await expect(page.getByRole('banner')).toContainText('Media at home');

  // The server stored both. It shows the name, and for the password only that it has one.
  await page.reload();
  await expect(page.getByLabel('Name of this install')).toHaveValue('Media at home');
  await expect(page.getByLabel('Registration password')).toHaveValue('');
  await expect(page.getByLabel('Registration password')).toHaveAttribute(
    'placeholder',
    'A value is stored',
  );
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
  await expect(menu.getByRole('link', { name: 'Ping' })).toBeVisible();
  // The tab "Settings" is for Admins and Co-Admins, and so are the settings.
  await expect(menu.getByRole('link', { name: 'Settings' })).toHaveCount(0);
  await page.getByLabel('Menu of the user').click();
  await expect(page.getByRole('banner').getByRole('link', { name: 'Settings' })).toHaveCount(0);
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
