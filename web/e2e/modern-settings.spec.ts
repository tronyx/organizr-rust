// The settings on the modern layout: every section of the side menu passes the contrast
// check of WCAG AA, in the dark colour set and in the light one (set on the page here).
// It runs after the walk of `demo.spec.ts`, on the same install, and leaves the install
// on the OG layout.

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

/**
 * Checks a settings section in each of its states: as it opens, with each tab of its form
 * (`.org-vtabs`) and with every accordion (`details`) of that tab open.
 */
async function checkSection(page: Page, where: string) {
  await checkContrast(page, where);
  const tabs = page.locator('.org-vtabs [role="tab"]');
  const count = await tabs.count();
  const openAccordions = async (state: string) => {
    const closed = page.locator('details.org-accordion:not([open]) > summary');
    const total = await closed.count();
    if (total === 0) {
      return;
    }
    for (let index = 0; index < total; index += 1) {
      await closed.first().click();
    }
    await expect(page.locator('details.org-accordion:not([open])')).toHaveCount(0);
    await checkContrast(page, `${state}, accordions open`);
  };
  if (count === 0) {
    await openAccordions(where);
    return;
  }
  for (let index = 0; index < count; index += 1) {
    const tab = tabs.nth(index);
    const name = ((await tab.textContent()) ?? '').trim();
    await tab.click();
    await expect(tab).toHaveAttribute('aria-selected', 'true');
    const state = `${where}, tab "${name}"`;
    if (index > 0) {
      await checkContrast(page, state);
    }
    await openAccordions(state);
  }
}

/** One dialog of each editor: the page, the button that opens it and its name. */
const DIALOGS = [
  { address: '#/settings/tabs', label: 'Tabs', button: 'Add a tab' },
  { address: '#/settings/categories', label: 'Categories', button: 'Add a category' },
  { address: '#/settings/groups', label: 'Manage Groups', button: 'Add a group' },
  { address: '#/settings/users', label: 'Manage Users', button: 'Add a user' },
] as const;

test('settings_sections_pass_contrast_on_modern_dark_and_light', async ({ page }) => {
  test.setTimeout(900_000);
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
        await checkSection(page, `${address} (${set})`);
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

test('the_side_menu_scrolls_inside_itself_on_a_long_page', async ({ page }) => {
  await logIn(page);
  const look = (layout: 'og' | 'modern') =>
    page.request.put('./api/v2/user/look', {
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colourSet: 'dark' },
    });

  try {
    expect((await look('modern')).ok()).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
    for (const [address, label] of [
      ['#/settings/general', 'Main'],
      ['#/settings/sso', 'SSO'],
    ] as const) {
      await page.goto('./');
      await page.goto(`./${address}`);
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
      await expect(page.locator('.m-page-heading h1')).toHaveText(label);
      await expect(page.locator('.org-tab-body [aria-label="Loading"]')).toHaveCount(0);

      const menu = page.locator('.m-settings-side');
      const donate = menu.getByRole('link', { name: 'Donate' });
      // The menu is no taller than the space under the top bar, and scrolls by itself.
      const sizes = await menu.evaluate((element) => ({
        client: element.clientHeight,
        scroll: element.scrollHeight,
        viewport: window.innerHeight,
      }));
      expect(sizes.client, `the menu on ${label}`).toBeLessThan(sizes.viewport - 52);
      expect(sizes.scroll, `the menu on ${label}`).toBeGreaterThan(sizes.client);

      // The page scrolls in the nearest ancestor of the menu that scrolls.
      const pageScroll = () =>
        menu.evaluate((element) => {
          let holder: Element | null = element.parentElement;
          while (holder && holder !== document.documentElement) {
            const overflow = getComputedStyle(holder).overflowY;
            if (/auto|scroll/.test(overflow) && holder.scrollHeight > holder.clientHeight) {
              break;
            }
            holder = holder.parentElement;
          }
          const scroller = holder ?? document.documentElement;
          return { top: scroller.scrollTop, max: scroller.scrollHeight - scroller.clientHeight };
        });
      const before = await pageScroll();
      await donate.scrollIntoViewIfNeeded();
      const where = await donate.evaluate((element) => {
        const box = element.getBoundingClientRect();
        return { top: box.top, bottom: box.bottom, height: window.innerHeight };
      });
      expect(where.top, `Donate on ${label}`).toBeGreaterThanOrEqual(52);
      expect(where.bottom, `Donate on ${label}`).toBeLessThanOrEqual(where.height);
      // The page itself did not scroll to the end to bring the link into view.
      const after = await pageScroll();
      expect(after.top - before.top, `the page scrolled on ${label}`).toBeLessThan(50);
      expect(after.max, `the page of ${label} scrolls`).toBeGreaterThan(100);
    }
  } finally {
    expect((await look('og')).ok()).toBe(true);
  }
});

test('the_note_of_style_and_themes_has_body_text_and_room_on_both_layouts', async ({ page }) => {
  await logIn(page);
  const look = (layout: 'og' | 'modern') =>
    page.request.put('./api/v2/user/look', {
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colourSet: 'dark' },
    });

  try {
    for (const layout of ['og', 'modern'] as const) {
      expect((await look(layout)).ok()).toBe(true);
      await page.goto('./#/settings/appearance');
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('data-layout', layout);
      await page.getByRole('tab', { name: 'Style & Themes' }).click();
      const panel = page.getByRole('tabpanel', { name: 'Style & Themes settings' });
      const note = panel.getByText(/This is the look that a person sees/);
      const layoutField = panel.getByRole('combobox', { name: 'Default layout' });
      await expect(layoutField).toBeVisible();

      // The note has the size of the body text, not the small size.
      const sizes = await note.evaluate((element) => ({
        note: getComputedStyle(element).fontSize,
        body: getComputedStyle(document.body).fontSize,
      }));
      expect(sizes.note, `the note on ${layout}`).toBe('14px');
      expect(sizes.note, `the note on ${layout}`).toBe(sizes.body);

      // The gap under the note is the gap between two fields of the form.
      const gaps = await note.evaluate((element) => {
        const grid = element.parentElement ?? document.body;
        const groups = [...grid.querySelectorAll(':scope > .org-form-group')];
        const boxes = groups.map((group) => group.getBoundingClientRect());
        const gap = (above: number) =>
          (boxes[above + 1]?.top ?? NaN) - (boxes[above]?.bottom ?? NaN);
        return { note: gap(0), fields: gap(1) };
      });
      expect(gaps.note, `the gap under the note on ${layout}`).toBeGreaterThanOrEqual(14);
      expect(gaps.note, `the gap under the note on ${layout}`).toBeCloseTo(gaps.fields, 0);
    }
  } finally {
    expect((await look('og')).ok()).toBe(true);
  }
});
