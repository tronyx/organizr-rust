// The modern layout: the shell with its grouped menu, user menu and phone drawer. It runs
// after the walk of `demo.spec.ts`, on the same install.

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
