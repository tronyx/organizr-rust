// The settings on the modern layout: every section of the side menu passes the contrast
// check of WCAG AA, in the dark colour set and in the light one (which people cannot pick
// yet, but whose styles exist). It runs after the walk of `demo.spec.ts`, on the same
// install, and leaves the install on the OG layout.

import type { Page } from '@playwright/test';

import { auditContrast, describeLowContrast } from './contrast';
import { expect, test } from './fixtures';
import { logIn } from './login';

type ColourSet = 'dark' | 'light';

/** The theme and colour set of the page, as the look of the person would set them. */
async function useColourSet(page: Page, set: ColourSet) {
  await page.evaluate((wanted) => {
    document.documentElement.dataset.theme = `m-organizr-${wanted}`;
    document.documentElement.dataset.colourSet = wanted;
  }, set);
  await expect(page.locator('html')).toHaveAttribute('data-theme', `m-organizr-${set}`);
}

/** Opens a settings page from its address and waits until its content has loaded. */
async function openSection(page: Page, address: string, label: string, set: ColourSet) {
  await page.goto(`./${address}`);
  await expect(page.locator('.m-page-heading h1')).toHaveText(label);
  await expect(page.locator('.org-tab-body [aria-label="Loading"]')).toHaveCount(0);
  await useColourSet(page, set);
}

/** Checks the contrast and records the texts it could not check. */
async function checkContrast(page: Page, where: string, within?: string) {
  // A page or a dialog fades in: the colours count once that is over. A spinner turns
  // for ever, and the scroll lock of the dialog follows the scrolling, not the time.
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          animation.playState !== 'running' ||
          !(animation.timeline instanceof DocumentTimeline) ||
          animation.effect?.getTiming().iterations === Infinity,
      ),
  );
  const audit = await auditContrast(page, within);
  if (audit.unknown.length > 0) {
    const lines = audit.unknown.map(
      (item) => `  "${item.text}" at ${item.selector}\n      because ${item.reason}`,
    );
    console.log(`Not checked, on a picture or gradient, on ${where}:\n${lines.join('\n')}`);
  }
  expect(audit.checked, `texts checked on ${where}`).toBeGreaterThan(0);
  expect.soft(audit.low, describeLowContrast(where, audit.low)).toEqual([]);
}

/** One dialog of each editor: the page, the button that opens it and its name. */
const DIALOGS = [
  { address: '#/settings/tabs', label: 'Tabs', button: 'Add a tab' },
  { address: '#/settings/categories', label: 'Categories', button: 'Add a category' },
  { address: '#/settings/groups', label: 'Manage Groups', button: 'Add a group' },
  { address: '#/settings/users', label: 'Manage Users', button: 'Add a user' },
] as const;

test('settings_sections_pass_contrast_on_modern_dark_and_light', async ({ page }) => {
  test.setTimeout(120_000);
  await logIn(page);
  const look = (layout: 'og' | 'modern') =>
    page.request.put('./api/v2/user/look', {
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colourSet: 'dark' },
    });

  try {
    expect((await look('modern')).ok()).toBe(true);
    await page.goto('./#/settings/tabs');
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');

    // The sections are the links of the side menu: all 22 of them.
    const links = page.getByRole('navigation', { name: 'Settings' }).getByRole('link');
    await expect(links).toHaveCount(22);
    const sections = await links.evaluateAll((anchors) =>
      anchors.map((anchor) => ({
        address: anchor.getAttribute('href') ?? '',
        label: anchor.textContent?.trim() ?? '',
      })),
    );

    for (const set of ['dark', 'light'] as const) {
      for (const { address, label } of sections) {
        await openSection(page, address, label, set);
        await checkContrast(page, `${address} (${set})`);
      }
      for (const { address, label, button } of DIALOGS) {
        await openSection(page, address, label, set);
        await page.getByRole('button', { name: button }).click();
        const dialog = page.getByRole('dialog', { name: button });
        await expect(dialog).toBeVisible();
        await checkContrast(page, `the dialog "${button}" (${set})`);
        await dialog.getByRole('button', { name: 'Cancel' }).click();
        await expect(dialog).toBeHidden();
      }
    }

    // On a phone the menu is a button that opens a sheet, and nothing covers the page.
    await page.setViewportSize({ width: 375, height: 812 });
    for (const set of ['dark', 'light'] as const) {
      for (const { address, label } of sections.slice(0, 3)) {
        await openSection(page, address, label, set);
        await expect(page.getByRole('navigation', { name: 'Settings' })).toHaveCount(0);
        const heading = page.locator('.m-page-heading h1');
        const covered = await heading.evaluate((element) => {
          const box = element.getBoundingClientRect();
          const top = document.elementFromPoint(box.left + 4, box.top + box.height / 2);
          return !(top && element.contains(top));
        });
        expect(covered, `the heading of ${address} is covered`).toBe(false);
        await checkContrast(page, `${address} on a phone (${set})`);

        await page.getByRole('button', { name: /Settings menu/ }).click();
        const sheet = page.getByRole('dialog', { name: 'Settings' });
        await expect(sheet.getByRole('navigation', { name: 'Settings' })).toBeVisible();
        await checkContrast(page, `the settings menu on a phone (${set})`, '[role="dialog"]');
        await page.keyboard.press('Escape');
        await expect(sheet).toBeHidden();
      }
    }
  } finally {
    // The install goes back to the OG layout for the tests after this one.
    expect((await look('og')).ok()).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
  }
});
