// The modern layout: the shell with its grouped menu, user menu and phone drawer. It runs
// after the walk of `demo.spec.ts`, on the same install.

import type { Page } from '@playwright/test';

import { expect, test } from './fixtures';

/** What the set-up of `demo.spec.ts` filled in. */
const ADMIN = { username: 'admin', password: 'the password of the browser tests' };

async function logIn(page: Page) {
  await page.goto('./#/login');
  await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in', exact: true }).click();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
}

test('the modern layout shows the grouped menu and the user menu, and breaks no policy', async ({
  page,
}) => {
  // The Content-Security-Policy reports a blocked script or style as an event of the page.
  const violations: string[] = [];
  await page.addInitScript(() => {
    document.addEventListener('securitypolicyviolation', (event) => {
      console.log(`policy-violation: ${event.violatedDirective} ${event.blockedURI}`);
    });
  });
  page.on('console', (message) => {
    if (message.text().startsWith('policy-violation:')) {
      violations.push(message.text());
    }
  });

  await logIn(page);
  const added = [
    await page.request.post('./api/v2/categories', {
      data: { category: 'Modern Media', image: '', category_id: 31 },
    }),
    await page.request.post('./api/v2/tabs', {
      data: { name: 'Modern Rox', url: '/api/v2/rox', image: '', enabled: 1, category_id: 31 },
    }),
  ];
  expect(added.map((response) => response.status())).toEqual([200, 200]);
  const look = (layout: 'og' | 'modern') =>
    page.request.put('./api/v2/user/look', {
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colourSet: 'dark' },
    });

  try {
    expect((await look('modern')).ok()).toBe(true);
    await page.reload();

    await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
    const menu = page.getByRole('navigation', { name: 'Tabs' });
    const group = menu.locator('details', { hasText: 'Modern Media' });
    await group.getByText('Modern Media').click();
    await group.getByRole('link', { name: 'Modern Rox' }).click();
    await expect(page).toHaveURL(/#Modern-Rox$/);
    await expect(page.frameLocator('iframe[title="Modern Rox"]').locator('body')).toContainText(
      'rox in socks!',
    );
    await expect(group.getByRole('link', { name: 'Modern Rox' })).toHaveAttribute(
      'aria-current',
      'page',
    );

    await page.getByRole('button', { name: 'Menu of the user' }).click();
    await expect(page.getByRole('menuitem', { name: 'Account Settings' })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: 'Lock Screen' })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: 'Logout' })).toBeVisible();
    await page.keyboard.press('Escape');

    // On a phone the menu is a drawer.
    await page.setViewportSize({ width: 390, height: 800 });
    await expect(menu).toBeHidden();
    await page.getByRole('button', { name: 'Show the menu of tabs' }).click();
    await expect(page.getByRole('dialog').getByRole('navigation', { name: 'Tabs' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeHidden();

    // The drawer slides in. A person who asked for less motion gets no animation.
    const animation = () =>
      page
        .locator('[data-slot="sheet-content"]')
        .evaluate((element) => getComputedStyle(element).animationName);
    await page.getByRole('button', { name: 'Show the menu of tabs' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    expect(await animation()).not.toBe('none');
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeHidden();
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.getByRole('button', { name: 'Show the menu of tabs' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    expect(await animation()).toBe('none');
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeHidden();
    await page.emulateMedia({ reducedMotion: 'no-preference' });
  } finally {
    // The install goes back to the OG layout for the tests after this one.
    expect((await look('og')).ok()).toBe(true);
  }
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-layout', 'og');

  expect(violations).toEqual([]);
});
