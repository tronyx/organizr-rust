// The login, register, recover, invite, lock, set-up and Account screens on the modern
// layout, with the Invites and Speed test pages and the media search of the top bar: each
// passes the contrast check of WCAG AA in the dark color set and in the light one, which
// the Admin default and the look of the person choose for real here. It runs after the
// walk of `demo.spec.ts`, on the same install. No request leaves the machine: the
// wallpapers are under `.invalid` and answered here, the media server is answered by
// fixtures, and every other request to another host is refused. The test leaves the
// settings and the look of the install as it found them.

import type { Page, Route } from '@playwright/test';

import { auditContrast, describeLowContrast } from './contrast';
import { expect, test } from './fixtures';
import { ADMIN, logIn } from './login';
import { ADDRESS } from './server';

type ColorSet = 'dark' | 'light';
const SETS: ColorSet[] = ['dark', 'light'];
const ORIGIN = `http://${ADDRESS}`;

/** The wallpapers of the login page: a bright and a dark one. */
const WALLPAPERS = {
  bright: 'https://wallpaper.invalid/bright.svg',
  dark: 'https://wallpaper.invalid/dark.svg',
} as const;
const PICTURES: Record<string, string> = {
  '/bright.svg':
    '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#fafafa"/></svg>',
  '/dark.svg':
    '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#050505"/></svg>',
};

/** The logo of the top bar: a wide picture, under `.invalid`, answered here. */
const LOGO = 'https://logo.invalid/wide.svg';
const LOGO_PICTURE =
  '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="60"><rect width="300" height="60" fill="#3366cc"/></svg>';

/** A picture of one transparent pixel, for the posters of the media search. */
const PIXEL = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
  'base64',
);

function envelope(data: unknown, result = 'success', message: string | null = null) {
  return JSON.stringify({ response: { result, message, data } });
}

/** An item of Plex, as the server builds it for the media search. */
function plexItem(title: string, more: Record<string, unknown> = {}) {
  return {
    type: 'movie',
    title,
    secondaryTitle: '2019',
    summary: `The story of ${title}.`,
    uid: `plex-${title}`,
    metadataKey: '815',
    imageURL: 'api/v2/homepage/image?source=plex&type=Primary&img=815&key=815-list',
    nowPlayingImageURL: 'api/v2/homepage/image?source=plex&type=Thumb&img=815&key=815-np',
    watched: 0,
    state: 'play',
    sessionType: 'Direct Playing',
    user: '',
    address: 'https://plex.invalid/web/item?id=815',
    ...more,
  };
}

const FOUND = {
  content: [
    plexItem('The Film'),
    plexItem('The Show', { type: 'tv' }),
    plexItem('No Picture', { imageURL: 'plugins/images/homepage/no-list.png' }),
  ],
};
const DETAILS = {
  content: [
    plexItem('The Film', {
      metadata: {
        summary: 'A long plot.',
        tagline: 'A made-up tagline',
        year: '2019',
        rating: '7.5',
        genres: ['Drama'],
        actors: [{ name: 'An Actor', role: 'The Hero' }],
      },
    }),
  ],
};

/** What the routes change in the launch data: set by each step of the test. */
interface LaunchPatch {
  wallpaper?: string;
  wizard?: boolean;
  search?: boolean;
  noLogo?: boolean;
  logo?: string;
  noAvatar?: boolean;
}

/**
 * Answers the wallpapers, the media server and the posters, changes the launch data as
 * `patch` says, refuses every other request to another host and lists it.
 */
async function routeEverything(page: Page, patch: LaunchPatch): Promise<string[]> {
  const outside: string[] = [];
  await page.route('**/*', async (route: Route) => {
    const request = route.request();
    const address = new URL(request.url());
    if (address.origin === 'https://wallpaper.invalid') {
      await route.fulfill({
        status: 200,
        contentType: 'image/svg+xml',
        body: PICTURES[address.pathname] ?? '',
      });
      return;
    }
    if (address.origin === 'https://logo.invalid') {
      await route.fulfill({ status: 200, contentType: 'image/svg+xml', body: LOGO_PICTURE });
      return;
    }
    // The avatar of the Admin comes from Gravatar: a pixel answers it here.
    if (address.origin !== ORIGIN && request.resourceType() === 'image') {
      await route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL });
      return;
    }
    if (address.origin !== ORIGIN) {
      outside.push(`${request.method()} ${request.url()}`);
      await route.abort('blockedbyclient');
      return;
    }
    const path = address.pathname.replace(/^\/api\/v2\//, '');
    if (path.startsWith('homepage/image') || address.pathname.includes('/homepage/no-list')) {
      await route.fulfill({ status: 200, contentType: 'image/png', body: PIXEL });
      return;
    }
    if (path.startsWith('homepage/plex/search/')) {
      await route.fulfill({ status: 200, contentType: 'application/json', body: envelope(FOUND) });
      return;
    }
    if (path === 'homepage/plex/metadata' && request.method() === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: envelope(DETAILS),
      });
      return;
    }
    // The answers of failed requests: no account is made and no e-mail is sent.
    const refusals: Record<string, string> = {
      'users/register': 'Registration Password was incorrect',
      'users/recover': 'The e-mail could not be sent',
      [`plugins/invites/FIXTURE`]: 'Code is invalid',
    };
    if (request.method() === 'POST' && path in refusals) {
      await route.fulfill({
        status: 422,
        contentType: 'application/json',
        body: envelope(null, 'error', refusals[path] ?? ''),
      });
      return;
    }
    if (path === 'launch' && request.method() === 'GET') {
      const answer = await route.fetch();
      const body = (await answer.json()) as {
        response: { data: Record<string, Record<string, unknown>> };
      };
      const data = body.response.data;
      const appearance = data.appearance ?? {};
      if (patch.wallpaper) {
        appearance.loginWallpaper = patch.wallpaper;
        appearance.randomMediaImage = '';
      }
      if (patch.noLogo) {
        appearance.useLogo = false;
        appearance.title = 'A rather long title of this Organizr install';
      }
      if (patch.noAvatar && data.user) {
        data.user.image = '';
      }
      if (patch.logo) {
        appearance.useLogo = true;
        appearance.logo = patch.logo;
      }
      if (patch.wizard && data.status) {
        data.status.status = 'wizard';
      }
      if (patch.search && data.settings) {
        const homepage = data.settings.homepage as Record<string, unknown>;
        homepage.search = { enabled: true, type: 'plex' };
      }
      await route.fulfill({ response: answer, body: JSON.stringify(body) });
      return;
    }
    await route.fallback();
  });
  return outside;
}

/** Waits until the fades are over, then checks the contrast of the page or of `within`. */
async function checkContrast(page: Page, where: string, within?: string, onPicture = false) {
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
  expect(audit.checked, `texts checked on ${where}`).toBeGreaterThan(0);
  expect.soft(audit.low, describeLowContrast(where, audit.low)).toEqual([]);
  if (onPicture) {
    // On a screen with a wallpaper every text sits on the card, not on the picture.
    expect.soft(audit.unknown, `texts on the wallpaper of ${where}`).toEqual([]);
  }
}

/** The page does not scroll sideways. */
async function expectNoSideScroll(page: Page, where: string) {
  const scroll = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    return root.scrollWidth - root.clientWidth;
  });
  expect(scroll, `${where} scrolls sideways`).toBe(0);
}

/**
 * Every control of the top bar, and the user button, ends inside the window. The bar is
 * `position: fixed`, so `expectNoSideScroll` cannot see it when it is too wide.
 */
async function expectBarFits(page: Page, where: string) {
  const bar = await page.evaluate(() => {
    const header = document.querySelector('header.org-header');
    const rect = (element: Element) => {
      const { left, right, width } = element.getBoundingClientRect();
      return { name: element.getAttribute('aria-label') ?? element.tagName, left, right, width };
    };
    const controls = [...(header?.querySelectorAll('button, a, img') ?? [])]
      .map(rect)
      .filter((control) => control.width > 0);
    const logo = header?.querySelector('.org-logo');
    const picture = logo?.querySelector('img');
    return {
      width: window.innerWidth,
      controls,
      user: header?.querySelector('[aria-label="Menu of the user"]')?.getBoundingClientRect().right,
      logoRight: logo?.getBoundingClientRect().right,
      pictureRight: picture?.getBoundingClientRect().right,
    };
  });
  expect(bar.controls.length, `controls of the top bar ${where}`).toBeGreaterThan(0);
  expect(bar.user, `the user button ${where}`).toBeDefined();
  for (const control of bar.controls) {
    expect.soft(control.left, `${control.name} ${where}: left edge`).toBeGreaterThanOrEqual(0);
    expect
      .soft(control.right, `${control.name} ${where}: right edge`)
      .toBeLessThanOrEqual(bar.width);
  }
  expect(bar.user ?? Infinity, `the user button ${where}`).toBeLessThanOrEqual(bar.width);
  if (bar.pictureRight !== undefined) {
    expect(bar.pictureRight, `the logo picture ${where}`).toBeLessThanOrEqual(
      (bar.logoRight ?? 0) + 0.5,
    );
  }
}

/** The contrast of the border of each radio that is not checked, against what is behind it. */
async function radioBorderRatios(page: Page, within: string): Promise<number[]> {
  return page.locator(within).evaluate((root) => {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const rgba = (color: string) => {
      if (!context) return [0, 0, 0, 0];
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = 'rgba(0, 0, 0, 0)';
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      const [r = 0, g = 0, b = 0, a = 0] = context.getImageData(0, 0, 1, 1).data;
      return [r, g, b, a / 255];
    };
    const behind = (element: Element) => {
      for (let current: Element | null = element.parentElement; current;) {
        const color = rgba(getComputedStyle(current).backgroundColor);
        if ((color[3] ?? 0) >= 1) return color;
        current = current.parentElement;
      }
      return [255, 255, 255, 1];
    };
    const luminance = (color: number[]) => {
      const channel = (value: number) => {
        const c = value / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      };
      const [r = 0, g = 0, b = 0] = color;
      return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    };
    return [...root.querySelectorAll('input.radio:not(:checked)')].map((radio) => {
      const back = behind(radio);
      const border = rgba(getComputedStyle(radio).borderTopColor);
      const alpha = border[3] ?? 1;
      const seen = [0, 1, 2].map((i) => (border[i] ?? 0) * alpha + (back[i] ?? 0) * (1 - alpha));
      const [high, low] = [luminance(seen), luminance(back)].sort((a, b) => b - a) as [
        number,
        number,
      ];
      return (high + 0.05) / (low + 0.05);
    });
  });
}

interface EffectiveLook {
  layout: 'og' | 'modern';
  theme: string;
  colorSet: string;
  source: 'user' | 'default';
}

async function readLook(page: Page): Promise<EffectiveLook> {
  const answer = await page.request.get('./api/v2/launch');
  expect(answer.ok(), 'the launch data is read').toBe(true);
  return ((await answer.json()) as { response: { data: { look: EffectiveLook } } }).response.data
    .look;
}

async function readDefaults(page: Page): Promise<Record<string, unknown>> {
  const answer = await page.request.get('./api/v2/config');
  expect(answer.ok(), 'the settings are read').toBe(true);
  const all = ((await answer.json()) as { response: { data: Record<string, unknown> } }).response
    .data;
  return { layout: all.layout, modernTheme: all.modernTheme, modernColorSet: all.modernColorSet };
}

async function put(page: Page, address: string, data: Record<string, unknown>) {
  const answer = await page.request.put(address, { data });
  expect(answer.ok(), `${address}: ${await answer.text()}`).toBe(true);
}

/** The Admin default and the look of the Admin: modern, in the color set. */
async function useModern(page: Page, set: ColorSet) {
  await put(page, './api/v2/config', {
    layout: 'modern',
    modernTheme: 'organizr',
    modernColorSet: set,
  });
  await put(page, './api/v2/user/look', { layout: 'modern', theme: 'organizr', colorSet: set });
}

/** Opens a page from its address and checks that the look of the server applies. */
async function open(page: Page, address: string, heading: string, set: ColorSet) {
  await page.goto(`./${address}`);
  await page.reload();
  await expect(page.getByRole('heading', { name: heading, exact: true })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
  await expect(page.locator('html')).toHaveAttribute('data-theme', `m-organizr-${set}`);
  await expect(page.locator('html')).toHaveAttribute('data-color-set', set);
}

test('login_lock_set_up_and_account_screens_pass_contrast_on_modern_dark_and_light', async ({
  page,
}) => {
  test.setTimeout(300_000);
  const patch: LaunchPatch = {};
  const outside = await routeEverything(page, patch);
  await logIn(page);
  const defaultsBefore = await readDefaults(page);
  const lookBefore = await readLook(page);

  try {
    for (const set of SETS) {
      await useModern(page, set);

      // The pages of a person who is logged in.
      await open(page, '#/account', 'Account', set);
      await checkContrast(page, `the Account page (${set})`);
      const pickerRatios = await radioBorderRatios(page, 'main');
      expect(pickerRatios.length, 'radios of the Account page').toBeGreaterThan(0);
      for (const ratio of pickerRatios) {
        expect(ratio, `a radio border on the Account page (${set})`).toBeGreaterThanOrEqual(3);
      }
      await open(page, '#/invites', 'Invites', set);
      await checkContrast(page, `the Invites page (${set})`);
      await open(page, '#/speedtest', 'Speed test', set);
      await checkContrast(page, `the Speed test page (${set})`);

      // The look popover of the top bar: its radios have the border of about 3:1.
      await page.getByRole('button', { name: 'Look' }).click();
      const popover = '[data-slot="popover-content"]';
      await expect(page.locator(popover).getByRole('radio', { name: 'Light' })).toBeVisible();
      await checkContrast(page, `the look popover (${set})`, popover);
      const popoverRatios = await radioBorderRatios(page, popover);
      expect(popoverRatios.length, 'radios of the look popover').toBeGreaterThan(0);
      for (const ratio of popoverRatios) {
        expect(ratio, `a radio border in the look popover (${set})`).toBeGreaterThanOrEqual(3);
      }
      await page.keyboard.press('Escape');
      await expect(page.locator(popover)).toHaveCount(0);

      // The media search of the top bar, with its results and the details of one.
      patch.search = true;
      await page.reload();
      await page.getByRole('button', { name: 'Search My Media' }).click();
      const search = page.getByRole('dialog', { name: 'Search My Media' });
      await search.getByRole('searchbox', { name: 'Search My Media' }).fill('film');
      await expect(search.getByRole('list', { name: 'Results in Plex' })).toBeVisible();
      await checkContrast(page, `the media search (${set})`);
      await search.getByRole('button', { name: 'The Film' }).click();
      await expect(page.getByText('A made-up tagline')).toBeVisible();
      await checkContrast(page, `the details of a search result (${set})`);
      patch.search = false;
      await page.reload();

      // The lock screen, with the error of a wrong password.
      patch.wallpaper = WALLPAPERS.bright;
      expect((await page.request.post('./api/v2/users/lock')).ok()).toBe(true);
      try {
        await page.reload();
        await expect(page.getByRole('heading', { name: 'The screen is locked' })).toBeAttached();
        await expect(page.locator('html')).toHaveAttribute('data-theme', `m-organizr-${set}`);
        await expect(page.locator('main.org-lock-screen')).toHaveCSS(
          'background-image',
          `url("${WALLPAPERS.bright}")`,
        );
        await checkContrast(page, `the lock screen (${set})`, undefined, true);
        await page.getByLabel('Password').fill('not the password');
        await page.getByRole('button', { name: 'Unlock' }).click();
        await expect(page.getByRole('alert')).toHaveText('Password Incorrect');
        await checkContrast(page, `the lock screen with an error (${set})`, undefined, true);
      } finally {
        await page.request.post('./api/v2/users/unlock', { data: { password: ADMIN.password } });
      }

      // The screens of a guest, which show the Admin default.
      await page.context().clearCookies();
      for (const [kind, wallpaper] of Object.entries(WALLPAPERS)) {
        patch.wallpaper = wallpaper;
        const where = `(${kind} wallpaper, ${set})`;
        await open(page, '#/login', 'Log in', set);
        await expect(page.locator('main.org-login-register')).toHaveCSS(
          'background-image',
          `url("${wallpaper}")`,
        );
        await checkContrast(page, `the login page ${where}`, undefined, true);
        await page.getByLabel('User name or e-mail address').fill(ADMIN.username);
        await page.getByLabel('Password').fill('not the password');
        await page.getByRole('button', { name: 'Log in', exact: true }).click();
        await expect(page.getByRole('alert')).toBeVisible();
        await checkContrast(page, `the login page with an error ${where}`, undefined, true);

        await open(page, '#/register', 'Make an account', set);
        await page.getByLabel('User name', { exact: true }).fill('nobody');
        await page.getByLabel('E-mail address').fill('nobody@example.invalid');
        await page.getByLabel('Password', { exact: true }).fill('a password of nobody');
        await page.getByLabel('Password, once more').fill('a password of nobody');
        await page.getByLabel('Registration password').fill('wrong');
        await page.getByRole('button', { name: 'Make the account' }).click();
        await expect(page.getByRole('alert')).toHaveText('Registration Password was incorrect');
        await checkContrast(page, `the register page with an error ${where}`, undefined, true);

        await open(page, '#/recover', 'Get a new password', set);
        await page.getByLabel('E-mail address', { exact: true }).fill('nobody@example.invalid');
        await page.getByRole('button', { name: 'Send the e-mail' }).click();
        await expect(page.getByRole('alert')).toHaveText('The e-mail could not be sent');
        await checkContrast(page, `the recover page with an error ${where}`, undefined, true);

        await page.goto('./?invite=FIXTURE');
        await expect(page.getByLabel('Invite code')).toHaveValue('FIXTURE');
        await expect(page.locator('html')).toHaveAttribute('data-theme', `m-organizr-${set}`);
        await checkContrast(page, `the invite page ${where}`, undefined, true);
        const inviteRatios = await radioBorderRatios(page, 'main');
        expect(inviteRatios.length, 'radios of the invite page').toBeGreaterThan(0);
        for (const ratio of inviteRatios) {
          expect(ratio, `a radio border on the invite page ${where}`).toBeGreaterThanOrEqual(3);
        }
        await page.getByLabel('Plex user name or e-mail address').fill('nobody@example.invalid');
        await page.getByRole('button', { name: 'Use the code' }).click();
        await expect(page.getByRole('alert')).toHaveText('Code is invalid');
        await checkContrast(page, `the invite page with an error ${where}`, undefined, true);
      }
      patch.wallpaper = undefined;

      // The set-up wizard shows only before the first account exists, and the walk of
      // `demo.spec.ts` made it: the launch data says that the install waits for it.
      patch.wizard = true;
      await open(page, '#/', 'Organizr Setup Wizard', set);
      await checkContrast(page, `the set-up wizard (${set})`);
      patch.wizard = false;
      // The wizard stays until the launch data is read again.
      await page.reload();
      await logIn(page);
    }

    // On a phone: no page scrolls sideways, the name of the user ends in an ellipsis,
    // and a title in text stays in the top bar.
    await page.setViewportSize({ width: 375, height: 812 });
    for (const set of SETS) {
      await useModern(page, set);
      patch.noLogo = true;
      await open(page, '#/account', 'Account', set);
      await expectNoSideScroll(page, `the Account page on a phone (${set})`);
      await checkContrast(page, `the Account page on a phone (${set})`);
      await expect(page.locator('.m-user-name')).toHaveCSS('text-overflow', 'ellipsis');
      const home = page.locator('header .org-logo > span');
      await expect(home).toBeVisible();
      await expect(home).toHaveCSS('text-overflow', 'ellipsis');
      const box = await home.boundingBox();
      expect(box?.width ?? 0, `the width of the title on a phone (${set})`).toBeGreaterThan(20);
      expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(375);
      // The long title really is cut.
      const cut = await home.evaluate((title) => title.scrollWidth - title.clientWidth);
      expect(cut, `the long title is cut on a phone (${set})`).toBeGreaterThan(0);
      await expectBarFits(page, `with a title in text (${set})`);

      // The top bar with the media search, a logo picture, and both.
      patch.search = true;
      await open(page, '#/account', 'Account', set);
      await expect(page.getByRole('button', { name: 'Search My Media' })).toBeVisible();
      await expectBarFits(page, `with a title in text and the media search (${set})`);
      patch.noLogo = false;
      patch.search = false;
      patch.logo = LOGO;
      await open(page, '#/account', 'Account', set);
      await expect(page.locator('header .org-logo > img')).toBeVisible();
      await expectBarFits(page, `with a logo picture (${set})`);
      patch.search = true;
      await open(page, '#/account', 'Account', set);
      await expect(page.getByRole('button', { name: 'Search My Media' })).toBeVisible();
      await expectBarFits(page, `with a logo picture and the media search (${set})`);
      patch.search = false;
      patch.logo = undefined;

      // Without an avatar the name keeps some room and ends in an ellipsis.
      patch.noAvatar = true;
      patch.search = true;
      patch.logo = LOGO;
      await open(page, '#/account', 'Account', set);
      await expect(page.getByRole('button', { name: 'Search My Media' })).toBeVisible();
      await expect(page.locator('.m-user img')).toHaveCount(0);
      await expect(page.locator('.m-user-name')).toHaveCSS('text-overflow', 'ellipsis');
      const nameWidth = await page.locator('.m-user-name').evaluate((name) => name.clientWidth);
      expect(nameWidth, `the name without an avatar on a phone (${set})`).toBeGreaterThan(0);
      await expectBarFits(page, `with no avatar, a logo picture and the media search (${set})`);
      patch.noAvatar = false;
      patch.search = false;
      patch.logo = undefined;

      patch.wallpaper = WALLPAPERS.dark;
      expect((await page.request.post('./api/v2/users/lock')).ok()).toBe(true);
      try {
        await page.reload();
        await expect(page.getByRole('heading', { name: 'The screen is locked' })).toBeAttached();
        await expect(page.locator('main.org-lock-screen')).toHaveCSS(
          'background-image',
          `url("${WALLPAPERS.dark}")`,
        );
        await expectNoSideScroll(page, `the lock screen on a phone (${set})`);
        await checkContrast(page, `the lock screen on a phone (${set})`, undefined, true);
      } finally {
        await page.request.post('./api/v2/users/unlock', { data: { password: ADMIN.password } });
      }

      await page.context().clearCookies();
      await open(page, '#/login', 'Log in', set);
      await expectNoSideScroll(page, `the login page on a phone (${set})`);
      await checkContrast(page, `the login page on a phone (${set})`, undefined, true);
      patch.wallpaper = undefined;
      await logIn(page);
    }
  } finally {
    Object.assign(patch, {
      wallpaper: undefined,
      wizard: false,
      search: false,
      noLogo: false,
      logo: undefined,
      noAvatar: false,
    });
    // Each step runs even when an earlier one fails; the failures are reported at the end.
    const failures: string[] = [];
    const restore = async (what: string, step: () => Promise<unknown>) => {
      try {
        await step();
      } catch (error) {
        failures.push(`${what}: ${error instanceof Error ? error.message : String(error)}`);
      }
    };
    await restore('the viewport', () => page.setViewportSize({ width: 1280, height: 720 }));
    await restore('the login', async () => {
      const answer = await page.request.post('./api/v2/login', {
        data: { username: ADMIN.username, password: ADMIN.password },
      });
      expect(answer.ok(), 'the Admin logs in again').toBe(true);
    });
    await restore('the unlock', () =>
      page.request.post('./api/v2/users/unlock', { data: { password: ADMIN.password } }),
    );
    await restore('the Admin defaults', () => put(page, './api/v2/config', defaultsBefore));
    await restore('the look of the Admin', () =>
      put(
        page,
        './api/v2/user/look',
        lookBefore.source === 'default'
          ? { layout: null }
          : { layout: lookBefore.layout, theme: lookBefore.theme, colorSet: lookBefore.colorSet },
      ),
    );
    expect.soft(failures, 'the restore of the install').toEqual([]);
  }
  expect(outside, 'requests to another host').toEqual([]);
});
