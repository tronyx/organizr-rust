// The modern layout: the shell with its grouped menu, user menu and phone drawer. It runs
// after the walk of `demo.spec.ts`, on the same install.

import type { Page } from '@playwright/test';

import { MODERN_THEMES } from '../src/lib/look';
import { auditContrast, describeLowContrast } from './contrast';
import { expect, test } from './fixtures';
import { logIn } from './login';

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
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colorSet: 'dark' },
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
    // The text reads as well as that of the OG menus: 14 px for the items, 12 px or more
    // for the name of the group.
    const sizes = await page.getByRole('menu').evaluate((menu) =>
      [...menu.querySelectorAll('[role="menuitem"], [data-slot="dropdown-menu-label"]')].map(
        (element) => ({
          label: element.getAttribute('data-slot') === 'dropdown-menu-label',
          size: parseFloat(getComputedStyle(element).fontSize),
        }),
      ),
    );
    expect(sizes.filter((one) => one.label)).toHaveLength(1);
    for (const { label, size } of sizes) {
      expect(size).toBeGreaterThanOrEqual(label ? 12 : 14);
    }
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

test('the modern menu collapses to a column of icons, and stays so in this browser', async ({
  page,
}) => {
  await logIn(page);
  const look = (layout: 'og' | 'modern', colorSet = 'dark') =>
    page.request.put('./api/v2/user/look', {
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colorSet },
    });
  const aside = page.locator('aside#m-menu');
  // The column and the page beside it move with a transition of their width and place. A
  // size counts once their transitions are over, not halfway through one.
  const settled = () =>
    page.evaluate(async () => {
      const moving = ['aside#m-menu', '.org-content'].flatMap((selector) =>
        [...document.querySelectorAll(selector)].flatMap((element) => element.getAnimations()),
      );
      await Promise.all(moving.map((animation) => animation.finished.catch(() => undefined)));
    });
  const width = async () => {
    await settled();
    return aside.evaluate((element) => element.getBoundingClientRect().width);
  };
  const contentLeft = async () => {
    await settled();
    return page.locator('.org-content').evaluate((element) => element.getBoundingClientRect().left);
  };
  const audit = async (where: string) => {
    // Wait until the column has its width and no tooltip shows.
    await page.mouse.move(900, 600);
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveCount(0);
    const result = await auditContrast(page);
    expect(result.checked, `texts checked on ${where}`).toBeGreaterThan(0);
    expect.soft(result.low, describeLowContrast(where, result.low)).toEqual([]);
  };

  try {
    expect((await look('modern')).ok()).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');

    const collapse = page.getByRole('button', { name: 'Collapse the menu' });
    await expect(collapse).toHaveAttribute('aria-expanded', 'true');
    await expect(collapse).toHaveAttribute('aria-controls', 'm-menu');
    // The column shrinks in a short time, and at once under reduced motion.
    const motion = () =>
      aside.evaluate((element) => {
        const style = getComputedStyle(element);
        return `${style.transitionProperty} ${style.transitionDuration}`;
      });
    expect(await motion()).toBe('width 0.15s');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    expect(await motion()).toMatch(/ 0s$/);
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    const wideWidth = await width();
    const wideLeft = await contentLeft();
    expect(wideWidth).toBeGreaterThan(200);

    await collapse.click();
    const expand = page.getByRole('button', { name: 'Expand the menu' });
    await expect(expand).toHaveAttribute('aria-expanded', 'false');
    await expect.poll(width).toBeLessThanOrEqual(80);
    await expect.poll(contentLeft).toBeLessThan(wideLeft);
    expect(await contentLeft()).toBeCloseTo(await width(), 0);
    await expect(page.getByRole('navigation', { name: 'Tabs' }).locator('summary')).toHaveCount(0);

    // The name of a tab shows as a tooltip, under the pointer and for the keyboard.
    const menu = page.getByRole('navigation', { name: 'Tabs' });
    const rox = menu.getByRole('link', { name: 'Modern Rox' });
    await rox.hover();
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveText('Modern Rox');
    await page.mouse.move(900, 600);
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveCount(0);
    const docs = page
      .getByRole('navigation', { name: 'Organizr' })
      .getByRole('link', { name: 'Organizr Docs' });
    await docs.focus();
    await page.keyboard.press('Shift+Tab');
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveCount(1);
    const focused = await page.evaluate(() => document.activeElement?.getAttribute('aria-label'));
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveText(focused ?? '');
    // The focus ring shows on the focused entry.
    const ring = await page.evaluate(() => {
      const style = getComputedStyle(document.activeElement as Element);
      return style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0;
    });
    expect(ring, 'a focus ring on the focused entry').toBe(true);

    // A tab of the column opens.
    await rox.click();
    await expect(page).toHaveURL(/#Modern-Rox$/);
    await expect(page.frameLocator('iframe[title="Modern Rox"]').locator('body')).toContainText(
      'rox in socks!',
    );
    await expect(rox).toHaveAttribute('aria-current', 'page');

    // The column stays after a reload.
    await page.reload();
    await expect(page.getByRole('button', { name: 'Expand the menu' })).toBeVisible();
    await expect.poll(width).toBeLessThanOrEqual(80);

    // No text of the page is too faint with the column, in dark and in light.
    await audit('the collapsed menu (organizr dark)');
    expect((await look('modern', 'light')).ok()).toBe(true);
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-color-set', 'light');
    await expect.poll(width).toBeLessThanOrEqual(80);
    await audit('the collapsed menu (organizr light)');

    await page.getByRole('button', { name: 'Expand the menu' }).click();
    await expect(page.getByRole('button', { name: 'Collapse the menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    await expect.poll(width).toBeCloseTo(wideWidth, 0);
    await expect.poll(contentLeft).toBeCloseTo(wideLeft, 0);
    await expect(menu.getByText('Modern Media')).toBeVisible();

    // On a phone there is no such button; the drawer still opens.
    await page.getByRole('button', { name: 'Collapse the menu' }).click();
    await page.setViewportSize({ width: 375, height: 800 });
    await expect(page.getByRole('button', { name: /(Collapse|Expand) the menu/ })).toHaveCount(0);
    await page.getByRole('button', { name: 'Show the menu of tabs' }).click();
    const drawer = page.getByRole('dialog');
    await expect(drawer.getByRole('navigation', { name: 'Tabs' })).toBeVisible();
    await expect(drawer.getByText('Modern Media')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(drawer).toBeHidden();
  } finally {
    await page.evaluate(() => localStorage.removeItem('organizr.modernMenuCollapsed'));
    expect((await look('og')).ok()).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
  }
});

test('the Look popover shows no tooltip when it closes after a click of the button', async ({
  page,
}) => {
  await logIn(page);
  const look = (layout: 'og' | 'modern') =>
    page.request.put('./api/v2/user/look', { data: { layout } });

  try {
    expect((await look('modern')).ok()).toBe(true);
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');

    const picker = page
      .locator('[data-slot="popover-content"]')
      .getByRole('radio', { name: 'Modern', exact: true });
    // The tooltip does show when the pointer rests on the button.
    await page.getByRole('button', { name: 'Look' }).hover();
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveCount(1);
    await page.getByRole('button', { name: 'Look' }).click();
    await expect(picker).toBeVisible();
    // Clicking the button again closes the popover, and the focus returns to the button.
    await page.getByRole('button', { name: 'Look' }).click();
    await expect(picker).toBeHidden();
    await page.waitForTimeout(500);
    await expect(page.locator('[data-slot="tooltip-content"]')).toHaveCount(0);
  } finally {
    expect((await look('og')).ok()).toBe(true);
  }
});

test('custom CSS of the Admin applies on the modern layout', async ({ page }) => {
  await logIn(page);
  const look = (layout: 'og' | 'modern') =>
    page.request.put('./api/v2/user/look', { data: { layout } });
  const css = '.org-header{outline:3px solid rgb(255,0,0)}';

  try {
    expect((await page.request.put('./api/v2/config', { data: { customCss: css } })).ok()).toBe(
      true,
    );
    expect((await look('modern')).ok()).toBe(true);
    await page.reload();

    await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
    await expect(page.locator('link#org-custom-css')).toHaveCount(1);
    await expect(page.locator('header.org-header')).toHaveCSS('outline-color', 'rgb(255, 0, 0)');
  } finally {
    await page.request.put('./api/v2/config', { data: { customCss: '' } });
    expect((await look('og')).ok()).toBe(true);
  }
});

test('the look popover chooses the light colors, and they stay after a reload', async ({
  page,
}) => {
  await logIn(page);
  const launch = await page.request.get('./api/v2/launch');
  const before = (
    (await launch.json()) as {
      response: {
        data: { look: { layout: string; theme: string; colorSet: string; source: string } };
      };
    }
  ).response.data.look;
  const html = page.locator('html');
  const choose = async (label: 'Light' | 'Dark') => {
    await page.getByRole('button', { name: 'Look' }).click();
    const radio = page
      .locator('[data-slot="popover-content"]')
      .getByRole('radio', { name: label, exact: true });
    await expect(radio).toBeEnabled();
    await radio.check();
    await expect(radio).toBeChecked();
    await page.keyboard.press('Escape');
  };

  try {
    const modern = await page.request.put('./api/v2/user/look', {
      data: { layout: 'modern', theme: 'organizr', colorSet: 'dark' },
    });
    expect(modern.ok()).toBe(true);
    await page.reload();
    await expect(html).toHaveAttribute('data-color-set', 'dark');

    await choose('Light');
    await expect(html).toHaveAttribute('data-theme', 'm-organizr-light');
    await expect(html).toHaveAttribute('data-color-set', 'light');
    await page.reload();
    await expect(html).toHaveAttribute('data-color-set', 'light');
    await expect(html).toHaveAttribute('data-theme', 'm-organizr-light');

    await choose('Dark');
    await expect(html).toHaveAttribute('data-color-set', 'dark');
    await page.reload();
    await expect(html).toHaveAttribute('data-color-set', 'dark');
    await expect(html).toHaveAttribute('data-theme', 'm-organizr-dark');
  } finally {
    const restored = await page.request.put('./api/v2/user/look', {
      data:
        before.source === 'default'
          ? { layout: null }
          : { layout: before.layout, theme: before.theme, colorSet: before.colorSet },
    });
    expect(restored.ok()).toBe(true);
  }
});

test('the color samples of the look picker take the colors of their own themes', async ({
  page,
}) => {
  await logIn(page);
  const launch = await page.request.get('./api/v2/launch');
  const before = (
    (await launch.json()) as {
      response: {
        data: { look: { layout: string; theme: string; colorSet: string; source: string } };
      };
    }
  ).response.data.look;

  /** The colors of a sample in the picker: its box, its border and its three dots. */
  const sampleOf = (within: string, name: string) =>
    page
      .locator(within)
      .getByRole('radio', { name, exact: true })
      .locator('xpath=ancestor::label[1]')
      .locator('.org-swatch')
      .evaluate((swatch) => {
        const style = getComputedStyle(swatch);
        const dots = [...swatch.querySelectorAll('.org-swatch-dot')].map(
          (dot) => getComputedStyle(dot).backgroundColor,
        );
        // The page color behind the picker: the first ancestor with a background.
        let behind = 'rgb(0, 0, 0)';
        for (let node = swatch.parentElement; node; node = node.parentElement) {
          const color = getComputedStyle(node).backgroundColor;
          if (color !== 'rgba(0, 0, 0, 0)' && color !== 'transparent') {
            behind = color;
            break;
          }
        }
        // A canvas turns any CSS color (also oklab) into sRGB, for the ratio of contrast.
        const context = document.createElement('canvas').getContext('2d');
        const luminance = (color: string) => {
          if (!context) return 0;
          context.clearRect(0, 0, 1, 1);
          context.fillStyle = color;
          context.fillRect(0, 0, 1, 1);
          const [r, g, b] = [...context.getImageData(0, 0, 1, 1).data].map((value) => {
            const c = value / 255;
            return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
          });
          return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
        };
        const [high, low] = [luminance(style.borderTopColor), luminance(behind)].sort(
          (x, y) => y - x,
        ) as [number, number];
        return {
          theme: swatch.getAttribute('data-theme'),
          box: style.backgroundColor,
          dots,
          edge: (high + 0.05) / (low + 0.05),
        };
      });

  try {
    // The modern layout: the samples in the popover of the top bar.
    const modern = await page.request.put('./api/v2/user/look', {
      data: { layout: 'modern', theme: 'organizr', colorSet: 'dark' },
    });
    expect(modern.ok()).toBe(true);
    await page.reload();
    await page.getByRole('button', { name: 'Look' }).click();
    const popover = '[data-slot="popover-content"]';
    await expect(
      page.locator(popover).getByRole('radio', { name: 'Plex', exact: true }),
    ).toBeVisible();
    const organizr = await sampleOf(popover, 'Organizr');
    const plex = await sampleOf(popover, 'Plex');
    expect(organizr.theme).toBe('m-organizr-dark');
    expect(plex.theme).toBe('m-plex-dark');
    // The primary dot (the second) of each sample is the primary color of its theme.
    expect(plex.dots[1], 'the primary dot of Plex').not.toBe(organizr.dots[1]);
    expect(plex.box, 'the box of Plex').not.toBe(organizr.box);
    for (const sample of [organizr, plex]) {
      expect(sample.dots).toHaveLength(3);
      expect(sample.edge, `the border of ${sample.theme}`).toBeGreaterThanOrEqual(3);
    }
    // A color set has its sample too.
    expect((await sampleOf(popover, 'Light')).theme).toBe('m-organizr-light');
    await page.keyboard.press('Escape');

    // The OG layout: the samples on the page Account.
    const og = await page.request.put('./api/v2/user/look', {
      data: { layout: 'og', theme: 'Organizr', colorSet: 'dark' },
    });
    expect(og.ok()).toBe(true);
    await page.goto('./#/account');
    // The page loads the look once; the reload brings the OG layout.
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-layout', 'og');
    const account = 'section[aria-labelledby="look-title"]';
    await expect(
      page.locator(account).getByRole('radio', { name: 'Nord', exact: true }),
    ).toBeVisible();
    const organizrOg = await sampleOf(account, 'Organizr');
    const nord = await sampleOf(account, 'Nord');
    expect(nord.theme).toBe('org-nord');
    expect(nord.dots[1], 'the primary dot of Nord').not.toBe(organizrOg.dots[1]);
    expect(nord.edge, 'the border of Nord').toBeGreaterThanOrEqual(3);
  } finally {
    const restored = await page.request.put('./api/v2/user/look', {
      data:
        before.source === 'default'
          ? { layout: null }
          : { layout: before.layout, theme: before.theme, colorSet: before.colorSet },
    });
    expect(restored.ok()).toBe(true);
  }
});

test('the arrow keys run through the themes of the look popover, and the focus stays', async ({
  page,
}) => {
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });

  try {
    expect((await look({ layout: 'modern', theme: 'organizr', colorSet: 'dark' })).ok()).toBe(true);
    await page.reload();
    await page.getByRole('button', { name: 'Look' }).click();
    const themes = page
      .locator('[data-slot="popover-content"]')
      .getByRole('group', { name: 'Theme', exact: true });
    const organizr = themes.getByRole('radio', { name: 'Organizr', exact: true });
    await organizr.focus();
    await expect(organizr).toBeChecked();

    // Three steps at once: Plex, Jellyfin, Emby. The focus never leaves the group.
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    const emby = themes.getByRole('radio', { name: 'Emby', exact: true });
    await expect(emby).toBeFocused();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'm-emby-dark');
    await expect(themes).toHaveAttribute('aria-busy', 'false');
    await expect(emby).toBeChecked();
    await expect(emby).toBeFocused();

    // One more step after the saves: the next theme, still with the keyboard.
    await page.keyboard.press('ArrowDown');
    const silo = themes.getByRole('radio', { name: 'Silo', exact: true });
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'm-silo-ember');
    await expect(themes).toHaveAttribute('aria-busy', 'false');
    await expect(silo).toBeFocused();
    const saved = (await (await page.request.get('./api/v2/launch')).json()) as {
      response: { data: { look: { theme: string } } };
    };
    expect(saved.response.data.look.theme).toBe('silo');
  } finally {
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});

test('the look popover fits a phone held sideways and scrolls inside', async ({ page }) => {
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });

  try {
    expect((await look({ layout: 'modern', theme: 'organizr', colorSet: 'dark' })).ok()).toBe(true);
    await page.setViewportSize({ width: 667, height: 375 });
    await page.reload();
    await page.getByRole('button', { name: 'Look' }).click();
    const panel = page.locator('[data-slot="popover-content"]');
    await expect(panel.getByRole('radio', { name: 'Modern', exact: true })).toBeVisible();

    // The panel ends inside the window, and its content scrolls.
    const box = await panel.boundingBox();
    expect(box).not.toBeNull();
    expect((box?.y ?? 0) + (box?.height ?? 0)).toBeLessThanOrEqual(375);
    const sizes = await panel.evaluate((element) => ({
      scroll: element.scrollHeight,
      client: element.clientHeight,
      overflow: getComputedStyle(element).overflowY,
    }));
    expect(sizes.overflow).toBe('auto');
    expect(sizes.scroll).toBeGreaterThan(sizes.client);

    // The last button can be reached, and it is then inside the window.
    const reset = panel.getByRole('button', { name: 'Use the default' });
    await reset.scrollIntoViewIfNeeded();
    await expect(reset).toBeInViewport({ ratio: 1 });
  } finally {
    await page.setViewportSize({ width: 1280, height: 720 });
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});

test('the shown tab of the column of icons has a bar of at least 3:1 in every modern theme', async ({
  page,
}) => {
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });
  // The tab Modern Rox comes from the first test of this file.

  /** The bar of the shown tab: its size, and its ratio of contrast against the column. */
  const bar = () =>
    page.locator('aside#m-menu a[aria-current="page"]').evaluate((link) => {
      const style = getComputedStyle(link, '::before');
      const aside = link.closest('aside') as Element;
      // A canvas turns any CSS color (also oklab) into sRGB, for the ratio of contrast.
      const context = document.createElement('canvas').getContext('2d');
      const luminance = (color: string) => {
        if (!context) return 0;
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = color;
        context.fillRect(0, 0, 1, 1);
        const [r, g, b] = [...context.getImageData(0, 0, 1, 1).data].map((value) => {
          const c = value / 255;
          return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
      };
      const [high, low] = [
        luminance(style.backgroundColor),
        luminance(getComputedStyle(aside).backgroundColor),
      ].sort((x, y) => y - x) as [number, number];
      // The bar sits at the left edge of the column.
      const left = Math.round(link.getBoundingClientRect().left + parseFloat(style.left));
      return {
        content: style.content,
        width: parseFloat(style.width),
        atEdge: left === Math.round(aside.getBoundingClientRect().left),
        ratio: (high + 0.05) / (low + 0.05),
      };
    });

  try {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.evaluate(() => localStorage.setItem('organizr.modernMenuCollapsed', 'true'));
    for (const theme of MODERN_THEMES) {
      for (const set of theme.colorSets) {
        expect((await look({ layout: 'modern', theme: theme.id, colorSet: set.id })).ok()).toBe(
          true,
        );
        await page.goto('./#Modern-Rox');
        await page.reload();
        await expect(page.locator('html')).toHaveAttribute('data-theme', `m-${theme.id}-${set.id}`);
        await expect(page.locator('aside#m-menu')).toHaveClass(/m-sidebar-icons/);
        const marker = await bar();
        const where = `the bar in m-${theme.id}-${set.id}`;
        expect(marker.content, where).toBe('""');
        expect(marker.width, where).toBeGreaterThanOrEqual(3);
        expect(marker.atEdge, where).toBe(true);
        expect.soft(marker.ratio, where).toBeGreaterThanOrEqual(3);
      }
    }
    // Expanded, the shown tab has no bar: its name and fill mark it.
    await page.evaluate(() => localStorage.removeItem('organizr.modernMenuCollapsed'));
    await page.reload();
    await expect(page.locator('aside#m-menu')).not.toHaveClass(/m-sidebar-icons/);
    expect((await bar()).content).toBe('none');
  } finally {
    await page.evaluate(() => localStorage.removeItem('organizr.modernMenuCollapsed'));
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});

test('on OG the sample of the theme of the install shows the colors of the Admin', async ({
  page,
}) => {
  await logIn(page);
  const launch = (await (await page.request.get('./api/v2/launch')).json()) as {
    response: { data: { theme: string; appearance: { buttonColor?: string } } };
  };
  const installTheme = launch.response.data.theme;
  const before = launch.response.data.appearance.buttonColor ?? '';
  const primaryDot = (name: string) =>
    page
      .locator('section[aria-labelledby="look-title"]')
      .getByRole('radio', { name, exact: true })
      .locator('xpath=ancestor::label[1]')
      .locator('.org-swatch-dot')
      .nth(1)
      .evaluate((dot) => getComputedStyle(dot).backgroundColor);

  try {
    expect(
      (await page.request.put('./api/v2/config', { data: { buttonColor: '#ff0000' } })).ok(),
    ).toBe(true);
    expect((await page.request.put('./api/v2/user/look', { data: { layout: 'og' } })).ok()).toBe(
      true,
    );
    await page.goto('./#/account');
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-layout', 'og');
    // The policy of the page lets the colors through the CSSOM.
    await expect.poll(() => primaryDot(installTheme)).toBe('rgb(255, 0, 0)');
    const other = installTheme === 'Nord' ? 'Plex' : 'Nord';
    expect(await primaryDot(other)).not.toBe('rgb(255, 0, 0)');
  } finally {
    await page.request.put('./api/v2/config', { data: { buttonColor: before } });
  }
});

/**
 * How the labels of the look picker inside `within` lay out: for each label, the lines of
 * its name and whether its own content or its box runs past its column or the picker.
 */
async function labelLayout(page: Page, within: string) {
  return page.locator(within).evaluate(async (scope) => {
    // The popover zooms in as it opens: its sizes count once that is over.
    await Promise.all(
      scope.getAnimations({ subtree: true }).map((animation) => animation.finished.catch(() => {})),
    );
    const grid = scope.querySelector('fieldset .grid');
    const gridBox = grid?.getBoundingClientRect();
    const labels = [...scope.querySelectorAll('fieldset label')];
    const rows = labels.map((label) => {
      const name = label.querySelector(':scope > span:not(.org-swatch)');
      const range = document.createRange();
      if (name?.firstChild) range.selectNodeContents(name.firstChild);
      const box = label.getBoundingClientRect();
      const lineHeight = parseFloat(getComputedStyle(name ?? label).lineHeight);
      return {
        name: name?.textContent?.trim() ?? '',
        lines: range.getClientRects().length,
        oneLineHigh: (name?.getBoundingClientRect().height ?? 0) <= lineHeight * 1.2,
        // The content of the label (radio, sample, name) stays inside the label.
        spills: label.scrollWidth > label.clientWidth + 1,
        box,
      };
    });
    // Two labels of the same row do not overlap.
    const overlaps = rows.flatMap((row, index) =>
      rows
        .slice(index + 1)
        .filter(
          (other) =>
            Math.abs(other.box.top - row.box.top) < 2 &&
            other.box.left < row.box.right - 0.5 &&
            row.box.left < other.box.right - 0.5,
        )
        .map((other) => `${row.name}/${other.name}`),
    );
    const outsideGrid = gridBox
      ? rows
          .filter((row) => row.box.right > gridBox.right + 0.5 || row.box.left < gridBox.left - 0.5)
          .map((row) => row.name)
      : [];
    // The labels end inside the padding of the picker, not on it.
    const style = getComputedStyle(scope);
    const outer = scope.getBoundingClientRect();
    const contentRight =
      outer.right - parseFloat(style.paddingRight) - parseFloat(style.borderRightWidth);
    const contentLeft =
      outer.left + parseFloat(style.paddingLeft) + parseFloat(style.borderLeftWidth);
    const pastPadding = rows
      .filter((row) => row.box.right > contentRight + 0.5 || row.box.left < contentLeft - 0.5)
      .map((row) => row.name);
    return {
      count: rows.length,
      pastPadding,
      notOneLine: rows.filter((row) => row.lines !== 1 || !row.oneLineHigh).map((row) => row.name),
      spills: rows.filter((row) => row.spills).map((row) => row.name),
      overlaps,
      outsideGrid,
      gridScrolls: grid ? grid.scrollWidth > grid.clientWidth + 1 : false,
      scopeScrolls: scope.scrollWidth > scope.clientWidth + 1,
      scopeBox: scope.getBoundingClientRect().toJSON() as DOMRect,
      viewport: window.innerWidth,
    };
  });
}

test('at the normal text size every name of the look picker is one line, whole', async ({
  page,
}) => {
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });

  try {
    expect((await look({ layout: 'modern', theme: 'organizr', colorSet: 'dark' })).ok()).toBe(true);
    for (const [width, height] of [
      [1280, 800],
      [667, 375],
    ] as const) {
      await page.setViewportSize({ width, height });
      await page.reload();
      await page.getByRole('button', { name: 'Look' }).click();
      const panel = '[data-slot="popover-content"]';
      await expect(
        page.locator(panel).getByRole('radio', { name: 'Nocturne', exact: true }),
      ).toBeVisible();
      const layout = await labelLayout(page, panel);
      const where = `the popover at ${width} x ${height}`;
      expect(layout.count, where).toBe(14);
      expect(layout.notOneLine, where).toEqual([]);
      expect(layout.spills, where).toEqual([]);
      expect(layout.overlaps, where).toEqual([]);
      expect(layout.pastPadding, where).toEqual([]);
      await page.keyboard.press('Escape');
    }

    // The Account page, on both layouts.
    await page.setViewportSize({ width: 1280, height: 800 });
    for (const layout of ['modern', 'og'] as const) {
      expect((await look({ layout })).ok()).toBe(true);
      await page.goto('./#/account');
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('data-layout', layout);
      const section = 'section[aria-labelledby="look-title"]';
      await expect(page.locator(section).getByRole('group', { name: 'Theme' })).toBeVisible();
      const found = await labelLayout(page, section);
      expect(found.count, `Account on ${layout}`).toBeGreaterThan(10);
      expect(found.notOneLine, `Account on ${layout}`).toEqual([]);
      expect(found.spills, `Account on ${layout}`).toEqual([]);
    }
  } finally {
    await page.setViewportSize({ width: 1280, height: 720 });
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});

test('at a text size of 200 % nothing spills out of the look popover', async ({ page }) => {
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });

  try {
    expect((await look({ layout: 'modern', theme: 'organizr', colorSet: 'dark' })).ok()).toBe(true);
    for (const [width, height] of [
      [1280, 900],
      [667, 375],
    ] as const) {
      await page.setViewportSize({ width, height });
      await page.reload();
      // A browser text size of 200 %: a root of 32 px instead of 16 px, set through the CSSOM.
      await page.evaluate(() => document.documentElement.style.setProperty('font-size', '32px'));
      await page.getByRole('button', { name: 'Look' }).click();
      const panel = '[data-slot="popover-content"]';
      await expect(
        page.locator(panel).getByRole('radio', { name: 'Nocturne', exact: true }),
      ).toBeVisible();
      const layout = await labelLayout(page, panel);
      const where = `the popover at 200 %, ${width} x ${height}`;
      expect(layout.count, where).toBe(14);
      expect(layout.spills, where).toEqual([]);
      expect(layout.overlaps, where).toEqual([]);
      expect(layout.outsideGrid, where).toEqual([]);
      expect(layout.pastPadding, where).toEqual([]);
      expect(layout.gridScrolls, where).toBe(false);
      expect(layout.scopeScrolls, where).toBe(false);
      // The popover itself stays inside the window.
      expect(layout.scopeBox.left, where).toBeGreaterThanOrEqual(0);
      expect(layout.scopeBox.right, where).toBeLessThanOrEqual(layout.viewport);
      // Each radio keeps its whole name.
      await expect(
        page.locator(panel).getByRole('radio', { name: 'Nocturne', exact: true }),
      ).toHaveAccessibleName('Nocturne');
      await page.keyboard.press('Escape');
      await page.evaluate(() => document.documentElement.style.removeProperty('font-size'));
    }
  } finally {
    await page.evaluate(() => document.documentElement.style.removeProperty('font-size'));
    await page.setViewportSize({ width: 1280, height: 720 });
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});

test('the border of every color sample is 3:1 on every offered modern theme', async ({ page }) => {
  test.setTimeout(120_000);
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });
  /** The lowest ratio of the border of a sample against the color behind it, and where. */
  const lowestEdge = () =>
    page.locator('[data-slot="popover-content"]').evaluate((panel) => {
      const context = document.createElement('canvas').getContext('2d');
      const luminance = (color: string) => {
        if (!context) return 0;
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = color;
        context.fillRect(0, 0, 1, 1);
        const [r, g, b] = [...context.getImageData(0, 0, 1, 1).data].map((value) => {
          const c = value / 255;
          return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
      };
      const ratios = [...panel.querySelectorAll('.org-swatch')].map((swatch) => {
        let behind = 'rgb(0, 0, 0)';
        for (let node = swatch.parentElement; node; node = node.parentElement) {
          const color = getComputedStyle(node).backgroundColor;
          if (color !== 'rgba(0, 0, 0, 0)' && color !== 'transparent') {
            behind = color;
            break;
          }
        }
        const [high, low] = [
          luminance(getComputedStyle(swatch).borderTopColor),
          luminance(behind),
        ].sort((x, y) => y - x) as [number, number];
        return {
          ratio: (high + 0.05) / (low + 0.05),
          sample: swatch.getAttribute('data-theme'),
        };
      });
      ratios.sort((x, y) => x.ratio - y.ratio);
      return { count: ratios.length, lowest: ratios[0] };
    });

  try {
    await page.setViewportSize({ width: 1280, height: 900 });
    for (const theme of MODERN_THEMES) {
      for (const set of theme.colorSets) {
        const where = `m-${theme.id}-${set.id}`;
        expect((await look({ layout: 'modern', theme: theme.id, colorSet: set.id })).ok()).toBe(
          true,
        );
        await page.reload();
        await expect(page.locator('html')).toHaveAttribute('data-theme', where);
        await page.getByRole('button', { name: 'Look' }).click();
        await expect(
          page.locator('[data-slot="popover-content"]').getByRole('radio', {
            name: theme.label,
            exact: true,
          }),
        ).toBeChecked();
        const edge = await lowestEdge();
        expect(edge.count, where).toBe(10 + theme.colorSets.length);
        expect
          .soft(edge.lowest?.ratio ?? 0, `the border of ${edge.lowest?.sample} on ${where}`)
          .toBeGreaterThanOrEqual(3);
        await page.keyboard.press('Escape');
      }
    }
  } finally {
    await page.setViewportSize({ width: 1280, height: 720 });
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});

test('the group label of the user menu reads well with the menu open, in dark and light', async ({
  page,
}) => {
  await logIn(page);
  const look = (data: Record<string, string>) => page.request.put('./api/v2/user/look', { data });

  try {
    for (const colorSet of ['dark', 'light']) {
      expect((await look({ layout: 'modern', theme: 'organizr', colorSet })).ok()).toBe(true);
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('data-color-set', colorSet);
      await page.getByRole('button', { name: 'Menu of the user' }).click();
      const label = page.locator('[data-slot="dropdown-menu-label"]');
      await expect(label).toHaveText('Admin');
      // The menu fades in: the colors count once that is over.
      await page
        .locator('[data-slot="dropdown-menu-content"]')
        .evaluate((menu) =>
          Promise.all(menu.getAnimations({ subtree: true }).map((animation) => animation.finished)),
        );
      // Only the open menu: its label and its entries.
      const audit = await auditContrast(page, '[data-slot="dropdown-menu-content"]');
      expect(audit.checked, `texts of the user menu (${colorSet})`).toBeGreaterThanOrEqual(4);
      expect(audit.low, describeLowContrast(`the user menu (${colorSet})`, audit.low)).toEqual([]);
      await page.keyboard.press('Escape');
      await expect(label).toBeHidden();
    }
  } finally {
    expect((await look({ layout: 'og' })).ok()).toBe(true);
  }
});
