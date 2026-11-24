// Logging in as the Admin that the set-up of `demo.spec.ts` made, for the tests of the
// modern layout.

import type { Page } from '@playwright/test';

import { expect } from './fixtures';

/** What the set-up of `demo.spec.ts` filled in. */
export const ADMIN = { username: 'admin', password: 'the password of the browser tests' };

export async function logIn(page: Page) {
  await page.goto('./#/login');
  await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
  await page.getByLabel('Password').fill(ADMIN.password);
  await page.getByRole('button', { name: 'Log in', exact: true }).click();
  await expect(page.getByLabel('Menu of the user')).toHaveText(ADMIN.username);
}
