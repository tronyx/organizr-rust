// The scene themes of the modern layout in the browser, in three tests.
//
// - On the first color set of every scene theme: no text sits on the scene on any page of
//   the shell; the scene runs at most 40 animations on the homepage, and stands still
//   while the page hides it (a frame tab, an opaque page such as the chat, the minimal
//   login of a guest).
// - A scene and its fonts load only for their theme: on OG and on a brand theme the
//   browser asks for no scene chunk and no scene font, and on each scene theme for its own
//   chunk and fonts and for none of another scene.
// - Every scene theme in each of its color sets passes the audit of the brand themes
//   (`modern-themes.spec.ts`): the homepage with the items of `homepage-fixtures.ts`, the
//   look popover, the Account page with its look picker, a settings section with a field
//   in focus, a settings dialog with its accordions open, the phone drawer, and the login
//   page of a guest with the Admin default on the theme. Each text passes WCAG AA and none
//   sits on a picture or gradient, the scene included. The scene runs, pauses in a hidden
//   tab, stands still for reduced motion and lets a click through to the page. A switch to
//   a brand theme removes the scene and its font at once.
//
// They run after the walk of `demo.spec.ts`, on the same install, and put the look and the
// settings of the Admin back. No request leaves the machine.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import type { Page, Request } from '@playwright/test';

import { MODERN_THEMES } from '../src/lib/look';
import { expect, test } from './fixtures';
import { allItemSettings, COMBINE_SWITCHES, unroutedRequests } from './homepage-fixtures';
import { logIn } from './login';
import {
  type AuditLook,
  checkContrast,
  edgesOf,
  expectEdges,
  expectLook,
  expectNoSideScroll,
  focusColors,
  keyboardFocus,
  lookToRestore,
  newGuest,
  openHomepage,
  openSettings,
  ORIGIN,
  put,
  readConfig,
  readLook,
  restoreAll,
  WALLPAPER,
} from './modern-audit';

/** The first color set of every scene theme. */
const SCENES = MODERN_THEMES.filter((theme) => theme.scene !== undefined).map((theme) => ({
  theme: theme.id,
  set: theme.colorSets[0]?.id ?? '',
}));

/** Every color set of every scene theme, light or dark by the `light` flag of the set. */
const SCENE_LOOKS: (AuditLook & { scene: string; light: boolean })[] = MODERN_THEMES.flatMap(
  (theme) =>
    theme.scene === undefined
      ? []
      : theme.colorSets.map((set) => ({
          theme: theme.id,
          set: set.id,
          setLabel: set.label,
          id: `m-${theme.id}-${set.id}`,
          scene: theme.scene ?? '',
          light: 'light' in set && set.light === true,
        })),
);

/** Every setting that the audit changes. */
const CHANGED = [...Object.keys(allItemSettings()), ...Object.values(COMBINE_SWITCHES)];

/** The file name of the scene component of a scene id: `nocturne` gives `NocturneScene`. */
const sceneModule = (scene: string) => `${scene.charAt(0).toUpperCase()}${scene.slice(1)}Scene`;

/**
 * The fonts of each scene, read from the `@fontsource` imports of its component: the build
 * names each font file after its package (`cinzel-latin-wght-normal-….woff2`).
 */
const SCENE_FONTS = new Map(
  MODERN_THEMES.flatMap((theme) => (theme.scene === undefined ? [] : [theme.scene])).map(
    (scene) => {
      const source = readFileSync(
        resolve(import.meta.dirname, '../src/scenes', `${sceneModule(scene)}.svelte`),
        'utf8',
      );
      const fonts = [...source.matchAll(/import '@fontsource(?:-variable)?\/([a-z0-9-]+)\//g)].map(
        (match) => match[1] ?? '',
      );
      return [scene, fonts] as const;
    },
  ),
);

/**
 * What a request asks for: the chunk of a scene (`js` or `css`), or a font of a scene (its
 * package name), or `null`. The build keeps the module name in the name of a chunk
 * (`NocturneScene-<hash>.js`) and the package name in the name of a font file.
 */
function sceneAsset(request: Request): { scene: string; what: string } | null {
  const path = new URL(request.url()).pathname;
  const chunk = /\/assets\/([A-Z][A-Za-z]*)Scene-[^/]*\.(js|css)$/.exec(path);
  if (chunk) {
    return { scene: (chunk[1] ?? '').toLowerCase(), what: chunk[2] ?? '' };
  }
  if (/\.woff2?$/.test(path)) {
    for (const [scene, fonts] of SCENE_FONTS) {
      const font = fonts.find((name) => path.startsWith(`/assets/${name}-`));
      if (font) return { scene, what: font };
    }
  }
  return null;
}

/** Every page of the shell that a test can reach on this install. */
const PAGES = [
  './#/',
  './#/settings/general',
  './#/account',
  './#/invites',
  './#/speedtest',
  './#/chat',
  './#/settings/plugins',
];

/** The fades of the page are over. The scene and a spinner turn for ever. */
async function settle(page: Page) {
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          animation.playState !== 'running' ||
          animation.effect?.getTiming().iterations === Infinity,
      ),
  );
}

/**
 * The visible texts whose first opaque background is the root or the body: they would sit
 * on the scene, which is behind every surface of the shell.
 */
async function textsOnTheScene(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const opaque = (color: string) => {
      if (!context) return false;
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = 'rgba(0, 0, 0, 0)';
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return (context.getImageData(0, 0, 1, 1).data[3] ?? 0) === 255;
    };
    const found: string[] = [];
    const seen = new Set<Element>();
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const element = node.parentElement;
      if (!element || seen.has(element) || !(node.textContent ?? '').trim()) continue;
      seen.add(element);
      if (element.closest('[aria-hidden="true"], .sr-only, svg, script, style')) continue;
      if (!element.checkVisibility({ visibilityProperty: true, opacityProperty: true })) continue;
      const box = element.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) continue;
      let holder: Element | null = element;
      while (holder) {
        const style = getComputedStyle(holder);
        if (opaque(style.backgroundColor) || style.backgroundImage !== 'none') break;
        holder = holder.parentElement;
      }
      if (!holder || holder === document.documentElement || holder === document.body) {
        const text = (node.textContent ?? '').trim().slice(0, 40);
        found.push(
          `"${text}" in ${element.tagName.toLowerCase()}.${[...element.classList].join('.')}`,
        );
      }
    }
    return found;
  });
}

/** The animations of the scene, by state. */
async function sceneAnimations(page: Page) {
  return page.evaluate(() => {
    const own = document.getAnimations().filter((animation) => {
      const target = (animation.effect as KeyframeEffect | null)?.target;
      return target instanceof Element && target.closest('.m-scene') !== null;
    });
    return {
      total: own.length,
      running: own.filter((animation) => animation.playState === 'running').length,
      paused: own.filter((animation) => animation.playState === 'paused').length,
    };
  });
}

/** Tells the page that the tab is hidden, or shown again. */
async function setHidden(page: Page, hidden: boolean) {
  await page.evaluate((value) => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => value });
    document.dispatchEvent(new Event('visibilitychange'));
  }, hidden);
}

/**
 * A spot of the window where the scene shows: the element there and each one above it to
 * the root has no opaque background and no picture. A click there must reach that element
 * of the page, not the scene. `null` when the cards cover the whole window.
 */
async function freeSpot(page: Page): Promise<{ x: number; y: number } | null> {
  return page.evaluate(() => {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const opaque = (color: string) => {
      if (!context) return false;
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = 'rgba(0, 0, 0, 0)';
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return (context.getImageData(0, 0, 1, 1).data[3] ?? 0) > 0;
    };
    for (let y = 80; y < window.innerHeight - 10; y += 24) {
      for (let x = 10; x < window.innerWidth - 10; x += 24) {
        const found = document.elementFromPoint(x, y);
        if (!found) continue;
        let clear = true;
        for (let at: Element | null = found; at && at !== document.documentElement;) {
          const style = getComputedStyle(at);
          if (opaque(style.backgroundColor) || style.backgroundImage !== 'none') {
            clear = false;
            break;
          }
          at = at.parentElement;
        }
        if (clear) return { x, y };
      }
    }
    return null;
  });
}

/** Clicks at a spot and tells which element the click reached. */
async function clickReaches(page: Page, spot: { x: number; y: number }) {
  await page.evaluate(() => {
    document.addEventListener(
      'click',
      (event) => {
        const target = event.target as Element;
        document.documentElement.dataset.clickReached = target.closest('.m-scene')
          ? 'the scene'
          : target.tagName.toLowerCase();
      },
      { once: true, capture: true },
    );
  });
  await page.mouse.click(spot.x, spot.y);
  const reached = await page.evaluate(() => {
    const value = document.documentElement.dataset.clickReached ?? 'nothing';
    delete document.documentElement.dataset.clickReached;
    return value;
  });
  return reached;
}

/**
 * Whether the drawer of the phone is opaque where it is: at its middle, an element between
 * the one under the point and the drawer itself has an opaque background.
 */
async function drawerCoversTheScene(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const drawer = document.querySelector('[role="dialog"]');
    if (!drawer) return false;
    const box = drawer.getBoundingClientRect();
    const found = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    if (!found || !drawer.contains(found)) return false;
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    for (let at: Element | null = found; at; at = at.parentElement) {
      if (context) {
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = 'rgba(0, 0, 0, 0)';
        context.fillStyle = getComputedStyle(at).backgroundColor;
        context.fillRect(0, 0, 1, 1);
        if ((context.getImageData(0, 0, 1, 1).data[3] ?? 0) === 255) return true;
      }
      if (at === drawer) break;
    }
    return false;
  });
}

test('the_scenes_keep_off_text_and_stop_when_unseen', async ({ page, browser }) => {
  test.setTimeout(180_000);
  expect(SCENES.length, 'scene themes').toBeGreaterThan(0);
  await logIn(page);
  const launch = await page.request.get('./api/v2/launch');
  const before = ((await launch.json()) as { response: { data: { look: Record<string, string> } } })
    .response.data.look;
  const config = (
    (await (await page.request.get('./api/v2/config')).json()) as {
      response: { data: Record<string, unknown> };
    }
  ).response.data;
  const configBefore = {
    layout: config.layout,
    modernTheme: config.modernTheme,
    modernColorSet: config.modernColorSet,
    minimalLoginScreen: config.minimalLoginScreen ?? false,
  };
  try {
    for (const { theme, set } of SCENES) {
      const id = `m-${theme}-${set}`;
      const answer = await page.request.put('./api/v2/user/look', {
        data: { layout: 'modern', theme, colorSet: set },
      });
      expect(answer.ok()).toBe(true);
      await page.goto('./#/');
      await page.reload();
      await expect(page.locator('html')).toHaveAttribute('data-theme', id);
      const scene = page.locator(`.m-scene.m-scene-${theme}`);
      await expect(scene).toHaveAttribute('aria-hidden', 'true');

      // No text on the scene, on any page of the shell.
      for (const address of PAGES) {
        await page.goto(address);
        await expect(scene).toBeAttached();
        await expect(page.locator('.m-page')).toBeVisible();
        await expect(page.locator('main [aria-label="Loading"], main .loading')).toHaveCount(0);
        await settle(page);
        expect
          .soft(await textsOnTheScene(page), `texts on the scene of ${id} at ${address}`)
          .toEqual([]);
      }

      // The homepage lets the scene run: at most 40 animations.
      await page.goto('./#/');
      await settle(page);
      const running = await sceneAnimations(page);
      expect(running.total, `animations of ${id}`).toBeGreaterThan(0);
      expect(running.total, `animations of ${id}`).toBeLessThanOrEqual(40);
      expect(running.running, `running animations of ${id}`).toBe(running.total);

      // A frame tab covers the page: the scene stands still.
      await page.goto('./#Ping');
      await expect(page.frameLocator('iframe[title="Ping"]').locator('body')).toContainText('pong');
      await expect(scene).toHaveClass(/m-scene-paused/);
      expect((await sceneAnimations(page)).running, `${id} under a frame tab`).toBe(0);
      // So does an opaque page, such as the chat.
      await page.goto('./#/chat');
      await expect(page.locator('.m-page')).toBeVisible();
      await expect(scene).toHaveClass(/m-scene-paused/);
      expect((await sceneAnimations(page)).running, `${id} behind the chat`).toBe(0);
      // Back on the homepage the scene runs again.
      await page.goto('./#/');
      await expect(scene).not.toHaveClass(/m-scene-paused/);
      expect((await sceneAnimations(page)).running, `${id} on the homepage`).toBe(running.total);

      // The minimal login screen of a guest, with the theme as the Admin default: its box
      // sits on an opaque page, so the scene stands still behind it.
      await put(page, './api/v2/config', {
        layout: 'modern',
        modernTheme: theme,
        modernColorSet: set,
        minimalLoginScreen: true,
      });
      const guestContext = await browser.newContext({ baseURL: page.url().split('#')[0] });
      try {
        const guest = await guestContext.newPage();
        await guest.goto('./#/login');
        await expect(guest.locator('html')).toHaveAttribute('data-theme', id);
        await expect(guest.getByRole('heading', { name: 'Log in', exact: true })).toBeAttached();
        await expect(guest.locator('main.org-login-register')).toHaveCount(0);
        const guestScene = guest.locator(`.m-scene.m-scene-${theme}`);
        await expect(guestScene).toHaveClass(/m-scene-paused/);
        const still = await sceneAnimations(guest);
        expect(still.total, `${id} behind the minimal login`).toBeGreaterThan(0);
        expect(still.running, `${id} behind the minimal login`).toBe(0);
      } finally {
        await guestContext.close();
        await put(page, './api/v2/config', configBefore);
      }
    }
  } finally {
    await page.request.put('./api/v2/user/look', {
      data:
        before?.source === 'default'
          ? { layout: null }
          : { layout: before?.layout, theme: before?.theme, colorSet: before?.colorSet },
    });
  }
});

test('scenes_and_their_fonts_load_only_for_their_theme', async ({ page, browser }) => {
  test.setTimeout(120_000);
  expect(SCENE_FONTS.size, 'scene themes').toBeGreaterThan(0);
  for (const [scene, fonts] of SCENE_FONTS) {
    expect(fonts, `the fonts that the scene ${scene} imports`).not.toEqual([]);
  }
  await logIn(page);
  const before = await readLook(page);
  const origin = ORIGIN;
  // OG, two brand themes, and the first color set of every scene theme: the scene and its
  // fonts are the same in every color set of a theme.
  const looks: { label: string; look: Record<string, string>; scene: string | null }[] = [
    { label: 'OG', look: { layout: 'og', theme: 'Organizr', colorSet: 'dark' }, scene: null },
    ...[
      ['organizr', 'dark'],
      ['plex', 'light'],
    ].map(([theme = '', colorSet = '']) => ({
      label: `${theme}/${colorSet}`,
      look: { layout: 'modern', theme, colorSet },
      scene: null,
    })),
    ...MODERN_THEMES.flatMap((theme) =>
      theme.scene === undefined
        ? []
        : [
            {
              label: `${theme.id}/${theme.colorSets[0]?.id ?? ''}`,
              look: { layout: 'modern', theme: theme.id, colorSet: theme.colorSets[0]?.id ?? '' },
              scene: theme.scene,
            },
          ],
    ),
  ];
  try {
    for (const { label, look, scene } of looks) {
      await put(page, './api/v2/user/look', look);
      // A context of its own for each look, with the session of the Admin: its cache is
      // empty, so every chunk and font that the look needs is asked for again.
      const context = await browser.newContext({
        baseURL: `${origin}/`,
        storageState: await page.context().storageState(),
      });
      try {
        const tab = await context.newPage();
        const problems: string[] = [];
        tab.on('pageerror', (error) => problems.push(`error in a script: ${error.message}`));
        await tab.route('**/*', async (route) => {
          if (new URL(route.request().url()).origin === origin) {
            await route.fallback();
          } else {
            await route.abort('blockedbyclient');
          }
        });
        const asked: string[] = [];
        tab.on('request', (request) => {
          const asset = sceneAsset(request);
          if (asset) asked.push(`${asset.scene}: ${asset.what}`);
        });
        const expected =
          scene === null
            ? []
            : [
                `${scene}: js`,
                `${scene}: css`,
                ...(SCENE_FONTS.get(scene) ?? []).map((font) => `${scene}: ${font}`),
              ];
        for (const address of [
          './#/',
          './#/settings/general',
          './#/settings/logs',
          './#/account',
        ]) {
          await tab.goto(address);
          await expect(tab.locator('html')).toHaveAttribute('data-layout', look.layout ?? '');
          if (address.endsWith('/logs')) {
            // The badges of the log levels: the only text of these pages in the font that
            // Circuit gives badges.
            await expect(tab.locator('table[aria-label="Log"] .badge').first()).toBeVisible();
          }
          await tab.waitForLoadState('networkidle');
          await tab.evaluate(() => document.fonts.ready);
        }
        if (scene !== null) {
          await expect(tab.locator(`.m-scene.m-scene-${scene}`)).toBeAttached();
          await expect
            .poll(() => [...new Set(asked)].sort(), `the scene assets of ${label}`)
            .toEqual([...expected].sort());
        } else {
          await expect(tab.locator('.m-scene')).toHaveCount(0);
          expect([...new Set(asked)], `the scene assets of ${label}`).toEqual([]);
        }
        const faces = await tab.evaluate(() =>
          [...document.fonts]
            .filter((face) => face.status !== 'unloaded')
            .map((face) => face.family.replace(/['"]/g, '')),
        );
        console.log(
          `${label}: ${[...new Set(asked)].join(', ') || 'no scene asset'}; faces ${[...new Set(faces)].join(', ')}`,
        );
        expect(problems, `the browser reported problems on ${label}`).toEqual([]);
      } finally {
        await context.close();
      }
    }
  } finally {
    await put(page, './api/v2/user/look', lookToRestore(before));
  }
});

test('every_scene_look_passes_the_audit', async ({ page, browser }) => {
  test.setTimeout(300_000);
  // Five scene themes of three color sets each; a later scene theme adds to them.
  expect(SCENE_LOOKS.length, 'color sets of the scene themes').toBeGreaterThanOrEqual(15);
  await logIn(page);
  const lookBefore = await readLook(page);
  const config = await readConfig(page);
  const defaultsBefore = {
    layout: config.layout,
    modernTheme: config.modernTheme,
    modernColorSet: config.modernColorSet,
    minimalLoginScreen: config.minimalLoginScreen ?? false,
  };
  const itemsBefore = Object.fromEntries(CHANGED.map((key) => [key, config[key] ?? '']));
  // The sessions of Overseerr and Ombi, which the search of the requests needs.
  await page
    .context()
    .addCookies(
      ['connect_sid', 'Auth'].map((name) => ({ name, value: 'fixture', url: `${ORIGIN}/` })),
    );
  const guest = await newGuest(browser);
  const timings: string[] = [];
  try {
    await put(page, './api/v2/config', {
      ...allItemSettings(),
      ...Object.fromEntries(Object.values(COMBINE_SWITCHES).map((key) => [key, false])),
    });

    for (const look of SCENE_LOOKS) {
      const started = Date.now();
      await put(page, './api/v2/user/look', {
        layout: 'modern',
        theme: look.theme,
        colorSet: look.set,
      });
      await put(page, './api/v2/config', {
        layout: 'modern',
        modernTheme: look.theme,
        modernColorSet: look.set,
        minimalLoginScreen: false,
      });

      // The homepage with every item, over the scene.
      await openHomepage(page, look);
      await expect(page.locator('html')).toHaveAttribute(
        'data-color-set',
        look.light ? 'light' : 'dark',
      );
      const scene = page.locator(`.m-scene.m-scene-${look.scene}`);
      await expect(scene).toHaveAttribute('aria-hidden', 'true');
      await expect(scene).toHaveCSS('pointer-events', 'none');
      const focus = await focusColors(page, look);
      await checkContrast(page, look, 'the homepage');
      expect
        .soft(await textsOnTheScene(page), `texts on the scene of the homepage (${look.id})`)
        .toEqual([]);

      // The scene runs, pauses in a hidden tab and runs again when the tab shows.
      const running = await sceneAnimations(page);
      expect(running.total, `animations of ${look.id}`).toBeGreaterThan(0);
      expect(running.running, `running animations of ${look.id}`).toBe(running.total);
      await setHidden(page, true);
      await expect(scene).toHaveClass(/m-scene-paused/);
      expect(await sceneAnimations(page), `${look.id} in a hidden tab`).toEqual({
        ...running,
        running: 0,
        paused: running.total,
      });
      await setHidden(page, false);
      await expect(scene).not.toHaveClass(/m-scene-paused/);
      expect((await sceneAnimations(page)).running, `${look.id} shown again`).toBe(running.total);

      // The look popover of the top bar.
      await page.getByRole('button', { name: 'Look' }).click();
      const popover = '[data-slot="popover-content"]';
      await expect(
        page.locator(popover).getByRole('radio', { name: look.setLabel, exact: true }),
      ).toBeChecked();
      await checkContrast(page, look, 'the look popover', popover);
      await page.keyboard.press('Escape');
      await expect(page.locator(popover)).toHaveCount(0);

      // The Account page, with its look picker.
      await page.goto('./#/account');
      await expect(page.getByRole('heading', { name: 'Account', exact: true })).toBeVisible();
      await expectLook(page, look);
      await expect(page.getByRole('radio', { name: look.setLabel, exact: true })).toBeChecked();
      await checkContrast(page, look, 'the Account page');

      // A settings section: its contrast, a click on a spot of the scene, which reaches the
      // page, and a field in focus.
      await openSettings(page, '#/settings/general', look);
      await checkContrast(page, look, 'the section General');
      const spot = await freeSpot(page);
      expect(spot, `a spot of the scene on the section General (${look.id})`).not.toBeNull();
      if (spot) {
        // The click must reach an element of the page: not the scene, and not lost.
        const reached = await clickReaches(page, spot);
        expect.soft(reached, `a click on the scene (${look.id})`).not.toBe('the scene');
        expect.soft(reached, `a click on the scene arrives (${look.id})`).not.toBe('nothing');
      }
      const field = page.getByRole('tabpanel').getByRole('textbox').first();
      await keyboardFocus(page, field);
      expectEdges(
        look,
        'the section General',
        { field: [await edgesOf(field), focus.field] },
        focus,
      );

      // A settings dialog with its accordions open.
      await openSettings(page, '#/settings/plugins', look);
      await page.getByRole('button', { name: 'Settings of PHP Mailer' }).click();
      const mailer = page.getByRole('dialog', { name: 'PHP Mailer Settings' });
      await expect(mailer).toBeVisible();
      await mailer.getByRole('tab', { name: 'Templates' }).click();
      const closed = mailer.locator('details.org-accordion:not([open]) > summary');
      const accordions = await closed.count();
      expect(accordions, 'accordions of the templates').toBeGreaterThan(0);
      for (let index = 0; index < accordions; index += 1) {
        await closed.first().click();
      }
      await expect(mailer.locator('details.org-accordion:not([open])')).toHaveCount(0);
      await checkContrast(page, look, 'the templates of PHP Mailer, accordions open');
      await page.keyboard.press('Escape');
      await expect(mailer).toBeHidden();

      // On a phone: nothing scrolls sideways, and the drawer covers the scene.
      await page.setViewportSize({ width: 375, height: 812 });
      try {
        await page.goto('./#Homepage');
        await expect(page.locator('section.org-item-calendar')).toBeVisible();
        await expect(scene).toBeAttached();
        await expectNoSideScroll(page, `the homepage on a phone (${look.id})`);
        await page.getByRole('button', { name: 'Show the menu of tabs' }).click();
        const drawer = page.getByRole('dialog');
        await expect(drawer.getByRole('navigation', { name: 'Tabs' })).toBeVisible();
        await checkContrast(page, look, 'the drawer on a phone', '[role="dialog"]');
        expect(await drawerCoversTheScene(page), `the drawer covers the scene (${look.id})`).toBe(
          true,
        );
        expect
          .soft(await textsOnTheScene(page), `texts on the scene with the drawer (${look.id})`)
          .toEqual([]);
        await expectNoSideScroll(page, `the drawer on a phone (${look.id})`);
        await page.keyboard.press('Escape');
        await expect(drawer).toBeHidden();
      } finally {
        await page.setViewportSize({ width: 1280, height: 720 });
      }

      // Reduced motion: the scene stays, and none of it moves.
      await page.emulateMedia({ reducedMotion: 'reduce' });
      try {
        await page.goto('./#/');
        await page.reload();
        await expect(scene).toBeAttached();
        await settle(page);
        const still = await sceneAnimations(page);
        expect(still.running, `running animations of ${look.id} with reduced motion`).toBe(0);
      } finally {
        await page.emulateMedia({ reducedMotion: 'no-preference' });
      }

      // The login page of a guest, in the Admin default: its card over the wallpaper, a
      // checked box, a field in focus and the primary button in focus.
      const visitor = guest.page;
      await visitor.goto('./#/login');
      await visitor.reload();
      await expect(visitor.getByRole('heading', { name: 'Log in', exact: true })).toBeAttached();
      await expectLook(visitor, look);
      await expect(visitor.locator('main.org-login-register')).toHaveCSS(
        'background-image',
        `url("${WALLPAPER}")`,
      );
      await expect(visitor.locator(`.m-scene.m-scene-${look.scene}`)).toHaveClass(/m-scene-paused/);
      await checkContrast(visitor, look, 'the login page of a guest');
      await visitor.getByRole('checkbox', { name: 'Stay logged in on this device' }).check();
      const box = await edgesOf(visitor.locator('main input.checkbox'));
      const username = visitor.locator('#login-username');
      await keyboardFocus(visitor, username);
      const typed = await edgesOf(username);
      const submit = visitor.locator('main button.btn-primary');
      await keyboardFocus(visitor, submit);
      expectEdges(
        look,
        'the login page of a guest',
        { box, field: [typed, focus.input], button: await edgesOf(submit) },
        focus,
      );
      timings.push(`${look.id}: ${((Date.now() - started) / 1000).toFixed(1)} s`);
    }

    // A switch to a brand theme in the look popover removes the scene and its font at once.
    const last = SCENE_LOOKS.at(-1);
    if (last) {
      await page.goto('./#/');
      await expect(page.locator(`.m-scene.m-scene-${last.scene}`)).toBeAttached();
      const heading = page.locator('.m-page-heading h1, .m-page h2, .m-page .card-title').first();
      const sceneFont = await heading.evaluate((element) => getComputedStyle(element).fontFamily);
      await page.getByRole('button', { name: 'Look' }).click();
      const popover = page.locator('[data-slot="popover-content"]');
      await popover.getByRole('radio', { name: 'Plex', exact: true }).check();
      await expect(page.locator('html')).toHaveAttribute('data-theme', /^m-plex-/);
      await expect(page.locator('.m-scene')).toHaveCount(0);
      expect(
        await heading.evaluate((element) => getComputedStyle(element).fontFamily),
        'the heading font after a switch to Plex',
      ).not.toBe(sceneFont);
      await page.keyboard.press('Escape');
    }
    expect(unroutedRequests(page), 'requests that no fixture answered').toEqual([]);
  } finally {
    const failures = await restoreAll([
      ['the guest', () => guest.context.close()],
      ['the routes', () => page.unrouteAll({ behavior: 'ignoreErrors' })],
      ['the viewport', () => page.setViewportSize({ width: 1280, height: 720 })],
      ['the motion', () => page.emulateMedia({ reducedMotion: 'no-preference' })],
      ['the items', () => put(page, './api/v2/config', itemsBefore)],
      ['the Admin defaults', () => put(page, './api/v2/config', defaultsBefore)],
      ['the look of the Admin', () => put(page, './api/v2/user/look', lookToRestore(lookBefore))],
    ]);
    console.log(`The scene looks:\n  ${timings.join('\n  ')}`);
    expect.soft(failures, 'the restore of the install').toEqual([]);
  }
  expect(guest.outside, 'requests of the guest to another host').toEqual([]);
  expect(guest.problems, 'the browser reported problems with the page of the guest').toEqual([]);
});
