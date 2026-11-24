// The scenes of the scene themes of the modern layout, in the browser, on the first colour
// set of every scene theme: no text sits on the scene on any page of the shell; the scene
// runs at most 40 animations on the homepage, pauses while the tab is hidden and while the
// page hides it (a frame tab, an opaque page such as the chat, the minimal login of a
// guest), and stands still for reduced motion. The full contrast audit of the scene themes
// is elsewhere; this checks what only a running scene shows. It runs after the walk of
// `demo.spec.ts`, on the same install, and puts the look and the settings of the Admin
// back.
//
// It also checks that a scene and its fonts load only for their theme: on OG and on a
// brand theme the browser asks for no scene chunk and no scene font, and on each scene
// theme for its own chunk and fonts and for none of another scene.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import type { Page, Request } from '@playwright/test';

import { MODERN_THEMES } from '../src/lib/look';
import { expect, test } from './fixtures';
import { logIn } from './login';
import { ADDRESS } from './server';

/** The first colour set of every scene theme. */
const SCENES = MODERN_THEMES.filter((theme) => theme.scene !== undefined).map((theme) => ({
  theme: theme.id,
  set: theme.colourSets[0]?.id ?? '',
}));

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
    const opaque = (colour: string) => {
      if (!context) return false;
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = 'rgba(0, 0, 0, 0)';
      context.fillStyle = colour;
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

async function put(page: Page, address: string, data: Record<string, unknown>) {
  const answer = await page.request.put(address, { data });
  expect(answer.ok(), `${address}: ${await answer.text()}`).toBe(true);
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
    modernColourSet: config.modernColourSet,
    minimalLoginScreen: config.minimalLoginScreen ?? false,
  };
  try {
    for (const { theme, set } of SCENES) {
      const id = `m-${theme}-${set}`;
      const answer = await page.request.put('./api/v2/user/look', {
        data: { layout: 'modern', theme, colourSet: set },
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

      // A hidden tab pauses every one of them, and showing it again resumes them.
      await setHidden(page, true);
      await expect(scene).toHaveClass(/m-scene-paused/);
      expect(await sceneAnimations(page), `${id} in a hidden tab`).toEqual({
        ...running,
        running: 0,
        paused: running.total,
      });
      await setHidden(page, false);
      await expect(scene).not.toHaveClass(/m-scene-paused/);
      expect((await sceneAnimations(page)).running).toBe(running.total);

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
        modernColourSet: set,
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

      // Reduced motion: no animation at all.
      await page.emulateMedia({ reducedMotion: 'reduce' });
      try {
        await page.reload();
        await expect(scene).toBeAttached();
        await settle(page);
        expect((await sceneAnimations(page)).total, `${id} with reduced motion`).toBe(0);
      } finally {
        await page.emulateMedia({ reducedMotion: 'no-preference' });
      }
    }
  } finally {
    await page.request.put('./api/v2/user/look', {
      data:
        before?.source === 'default'
          ? { layout: null }
          : { layout: before?.layout, theme: before?.theme, colourSet: before?.colourSet },
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
  const launch = await page.request.get('./api/v2/launch');
  const before = ((await launch.json()) as { response: { data: { look: Record<string, string> } } })
    .response.data.look;
  const origin = `http://${ADDRESS}`;
  // OG, two brand themes, and the first colour set of every scene theme: the scene and its
  // fonts are the same in every colour set of a theme.
  const looks: { label: string; look: Record<string, string>; scene: string | null }[] = [
    { label: 'OG', look: { layout: 'og', theme: 'Organizr', colourSet: 'dark' }, scene: null },
    ...[
      ['organizr', 'dark'],
      ['plex', 'light'],
    ].map(([theme = '', colourSet = '']) => ({
      label: `${theme}/${colourSet}`,
      look: { layout: 'modern', theme, colourSet },
      scene: null,
    })),
    ...MODERN_THEMES.flatMap((theme) =>
      theme.scene === undefined
        ? []
        : [
            {
              label: `${theme.id}/${theme.colourSets[0]?.id ?? ''}`,
              look: { layout: 'modern', theme: theme.id, colourSet: theme.colourSets[0]?.id ?? '' },
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
    await page.request.put('./api/v2/user/look', {
      data:
        before?.source === 'default'
          ? { layout: null }
          : { layout: before?.layout, theme: before?.theme, colourSet: before?.colourSet },
    });
  }
});
