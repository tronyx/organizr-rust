// The modern layout: the shell with its grouped menu, user menu and phone drawer. It runs
// after the walk of `demo.spec.ts`, on the same install.

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
  const look = (layout: 'og' | 'modern', colourSet = 'dark') =>
    page.request.put('./api/v2/user/look', {
      data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colourSet },
    });
  const aside = page.locator('aside#m-menu');
  const width = () => aside.evaluate((element) => element.getBoundingClientRect().width);
  const contentLeft = () =>
    page.locator('.org-content').evaluate((element) => element.getBoundingClientRect().left);
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
    await expect(page.locator('html')).toHaveAttribute('data-colour-set', 'light');
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

test('the look popover chooses the light colours, and they stay after a reload', async ({
  page,
}) => {
  await logIn(page);
  const launch = await page.request.get('./api/v2/launch');
  const before = (
    (await launch.json()) as {
      response: {
        data: { look: { layout: string; theme: string; colourSet: string; source: string } };
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
      data: { layout: 'modern', theme: 'organizr', colourSet: 'dark' },
    });
    expect(modern.ok()).toBe(true);
    await page.reload();
    await expect(html).toHaveAttribute('data-colour-set', 'dark');

    await choose('Light');
    await expect(html).toHaveAttribute('data-theme', 'm-organizr-light');
    await expect(html).toHaveAttribute('data-colour-set', 'light');
    await page.reload();
    await expect(html).toHaveAttribute('data-colour-set', 'light');
    await expect(html).toHaveAttribute('data-theme', 'm-organizr-light');

    await choose('Dark');
    await expect(html).toHaveAttribute('data-colour-set', 'dark');
    await page.reload();
    await expect(html).toHaveAttribute('data-colour-set', 'dark');
    await expect(html).toHaveAttribute('data-theme', 'm-organizr-dark');
  } finally {
    const restored = await page.request.put('./api/v2/user/look', {
      data:
        before.source === 'default'
          ? { layout: null }
          : { layout: before.layout, theme: before.theme, colourSet: before.colourSet },
    });
    expect(restored.ok()).toBe(true);
  }
});

test('the colour samples of the look picker take the colours of their own themes', async ({
  page,
}) => {
  await logIn(page);
  const launch = await page.request.get('./api/v2/launch');
  const before = (
    (await launch.json()) as {
      response: {
        data: { look: { layout: string; theme: string; colourSet: string; source: string } };
      };
    }
  ).response.data.look;

  /** The colours of a sample in the picker: its box, its border and its three dots. */
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
        // The page colour behind the picker: the first ancestor with a background.
        let behind = 'rgb(0, 0, 0)';
        for (let node = swatch.parentElement; node; node = node.parentElement) {
          const colour = getComputedStyle(node).backgroundColor;
          if (colour !== 'rgba(0, 0, 0, 0)' && colour !== 'transparent') {
            behind = colour;
            break;
          }
        }
        // A canvas turns any CSS colour (also oklab) into sRGB, for the ratio of contrast.
        const context = document.createElement('canvas').getContext('2d');
        const luminance = (colour: string) => {
          if (!context) return 0;
          context.clearRect(0, 0, 1, 1);
          context.fillStyle = colour;
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
      data: { layout: 'modern', theme: 'organizr', colourSet: 'dark' },
    });
    expect(modern.ok()).toBe(true);
    await page.reload();
    await page.getByRole('button', { name: 'Look' }).click();
    const popover = '[data-slot="popover-content"]';
    await expect(page.locator(popover).getByRole('radio', { name: 'Plex' })).toBeVisible();
    const organizr = await sampleOf(popover, 'Organizr');
    const plex = await sampleOf(popover, 'Plex');
    expect(organizr.theme).toBe('m-organizr-dark');
    expect(plex.theme).toBe('m-plex-dark');
    // The primary dot (the second) of each sample is the primary colour of its theme.
    expect(plex.dots[1], 'the primary dot of Plex').not.toBe(organizr.dots[1]);
    expect(plex.box, 'the box of Plex').not.toBe(organizr.box);
    for (const sample of [organizr, plex]) {
      expect(sample.dots).toHaveLength(3);
      expect(sample.edge, `the border of ${sample.theme}`).toBeGreaterThanOrEqual(3);
    }
    // A colour set has its sample too.
    expect((await sampleOf(popover, 'Light')).theme).toBe('m-organizr-light');
    await page.keyboard.press('Escape');

    // The OG layout: the samples on the page Account.
    const og = await page.request.put('./api/v2/user/look', {
      data: { layout: 'og', theme: 'Organizr', colourSet: 'dark' },
    });
    expect(og.ok()).toBe(true);
    await page.goto('./#/account');
    // The page loads the look once; the reload brings the OG layout.
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-layout', 'og');
    const account = 'section[aria-labelledby="look-title"]';
    await expect(page.locator(account).getByRole('radio', { name: 'Nord' })).toBeVisible();
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
          : { layout: before.layout, theme: before.theme, colourSet: before.colourSet },
    });
    expect(restored.ok()).toBe(true);
  }
});
