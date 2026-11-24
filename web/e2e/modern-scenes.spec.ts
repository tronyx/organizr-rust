// The scenes of the scene themes of the modern layout, in the browser, on the first colour
// set of every scene theme: no text sits on the scene on any page of the shell; the scene
// runs at most 40 animations, pauses while the tab is hidden and while a frame tab covers
// the page, and stands still for reduced motion. The full contrast audit of the scene
// themes is elsewhere; this checks what only a running scene shows. It runs after the walk
// of `demo.spec.ts`, on the same install, and puts the look of the Admin back.

import type { Page } from '@playwright/test';

import { MODERN_THEMES } from '../src/lib/look';
import { expect, test } from './fixtures';
import { logIn } from './login';

/** The first colour set of every scene theme. */
const SCENES = MODERN_THEMES.filter((theme) => theme.scene !== undefined).map((theme) => ({
  theme: theme.id,
  set: theme.colourSets[0]?.id ?? '',
}));

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

test('the_scenes_keep_off_text_and_stop_when_unseen', async ({ page }) => {
  test.setTimeout(180_000);
  expect(SCENES.length, 'scene themes').toBeGreaterThan(0);
  await logIn(page);
  const launch = await page.request.get('./api/v2/launch');
  const before = ((await launch.json()) as { response: { data: { look: Record<string, string> } } })
    .response.data.look;
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
      await page.goto('./#/');
      await expect(scene).not.toHaveClass(/m-scene-paused/);

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
