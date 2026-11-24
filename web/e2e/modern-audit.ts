// What the browser audits of the themes of the modern layout share: the contrast check with
// its known exceptions, the edges and the focus of controls, the homepage with the items of
// `homepage-fixtures.ts`, a guest that gets the wallpaper of the test, and the restore of
// the install. `modern-themes.spec.ts` audits the brand themes with it and
// `modern-scenes.spec.ts` the scene themes.

import type { Browser, BrowserContext, Locator, Page, Route } from '@playwright/test';

import { auditContrast, describeLowContrast } from './contrast';
import { expect } from './fixtures';
import { HOMEPAGE_FIXTURES, withHomepageFixtures } from './homepage-fixtures';
import { ADDRESS } from './server';

export const ORIGIN = `http://${ADDRESS}`;

/** A theme and colour set of the modern layout, as an audit goes through them. */
export interface AuditLook {
  theme: string;
  set: string;
  setLabel: string;
  /** The id of the daisyUI theme: `m-<theme>-<set>`. */
  id: string;
}

/** The kinds that show on the homepage when every item is on and none is combined. */
export const KINDS = HOMEPAGE_FIXTURES.map((fixture) => fixture.kind);

/** The wallpaper of the login page: dark, so the card shows its own colours. */
export const WALLPAPER = 'https://wallpaper.invalid/dark.svg';
const WALLPAPER_PICTURE =
  '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#050505"/></svg>';

/** The fades of the page and of its dialogs are over. A spinner and a scene turn for ever. */
export async function waitForAnimations(page: Page) {
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
}

/**
 * Checks the contrast of the page, of its open dialog or of `within`. A text on a picture
 * or gradient fails the test, but for two:
 * - the text of a primary button of Jellyfin (`.btn-primary`, and `.org-form-button`, which
 *   the same rule paints), which sits on the gradient of the theme in the primary text
 *   colour. The unit test `the_text_of_the_jellyfin_gradient_meets_aa_on_both_ends`
 *   (`src/lib/look.test.ts`) checks that colour against both ends of the gradient;
 * - the text of a bookmark tile, whose gradient and text colours the Admin chose for the
 *   tile (inline in its `style`): they are the same on every theme.
 */
export async function checkContrast(page: Page, look: AuditLook, where: string, within?: string) {
  await waitForAnimations(page);
  const audit = await auditContrast(page, within);
  const label = `${where} (${look.id})`;
  expect(audit.checked, `texts checked on ${label}`).toBeGreaterThan(0);
  expect.soft(audit.low, describeLowContrast(label, audit.low)).toEqual([]);
  const holders = audit.unknown.map((item) => item.reason.split(' has ')[0] ?? '');
  const kinds = await page.evaluate((paths) => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--color-primary-content)';
    document.body.append(probe);
    const primaryText = getComputedStyle(probe).color;
    probe.remove();
    return paths.map((path) => {
      try {
        const found = [...document.querySelectorAll(path)];
        const all = (test: (element: Element) => boolean) => found.length > 0 && found.every(test);
        const gradient = (element: Element) =>
          getComputedStyle(element).backgroundImage.startsWith('linear-gradient(');
        if (
          all(
            (element) =>
              element.matches('.btn-primary, .org-form-button') &&
              gradient(element) &&
              getComputedStyle(element).color === primaryText,
          )
        ) {
          return 'primary';
        }
        if (
          all(
            (element) =>
              element instanceof HTMLElement &&
              element.style.background.includes('gradient') &&
              element.closest('section.org-item-bookmarks') !== null,
          )
        ) {
          return 'tile';
        }
        return 'other';
      } catch {
        return 'other';
      }
    });
  }, holders);
  const unchecked = audit.unknown.filter(
    (_, index) =>
      !(look.theme === 'jellyfin' && kinds[index] === 'primary') && kinds[index] !== 'tile',
  );
  expect.soft(unchecked, `texts on a picture or gradient on ${label}`).toEqual([]);
}

/** The page does not scroll sideways. */
export async function expectNoSideScroll(page: Page, where: string) {
  const scroll = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    return root.scrollWidth - root.clientWidth;
  });
  expect(scroll, `${where} scrolls sideways`).toBe(0);
}

/** What a control shows of itself, as contrast ratios against what is behind it. */
export interface Edges {
  /** The fill of the control (its background colour, when it is mostly opaque). */
  fill: number | null;
  /** Its border. */
  border: number;
  /** What marks the focus: the outline, or the border when there is no outline. */
  ring: number;
  ringColour: string;
  focusVisible: boolean;
}

/** The contrast of the fill, the border and the outline of a control against the card. */
export async function edgesOf(control: Locator): Promise<Edges> {
  return control.evaluate((control) => {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const rgba = (colour: string) => {
      if (!context) return [0, 0, 0, 0];
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = 'rgba(0, 0, 0, 0)';
      context.fillStyle = colour;
      context.fillRect(0, 0, 1, 1);
      const [r = 0, g = 0, b = 0, a = 0] = context.getImageData(0, 0, 1, 1).data;
      return [r, g, b, a / 255];
    };
    const behind = (() => {
      for (let current = control.parentElement; current; current = current.parentElement) {
        const colour = rgba(getComputedStyle(current).backgroundColor);
        if ((colour[3] ?? 0) >= 1) return colour;
      }
      return [255, 255, 255, 1];
    })();
    const luminance = (colour: number[]) => {
      const channel = (value: number) => {
        const c = value / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      };
      const [r = 0, g = 0, b = 0] = colour;
      return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    };
    const ratio = (colour: string) => {
      const own = rgba(colour);
      const alpha = own[3] ?? 1;
      const seen = [0, 1, 2].map((i) => (own[i] ?? 0) * alpha + (behind[i] ?? 0) * (1 - alpha));
      const [high, low] = [luminance(seen), luminance(behind)].sort((a, b) => b - a) as [
        number,
        number,
      ];
      return Math.round(((high + 0.05) / (low + 0.05)) * 100) / 100;
    };
    const style = getComputedStyle(control);
    const hasOutline = style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0;
    return {
      fill: (rgba(style.backgroundColor)[3] ?? 0) >= 0.5 ? ratio(style.backgroundColor) : null,
      border: ratio(style.borderTopColor),
      ring: ratio(hasOutline ? style.outlineColor : style.borderTopColor),
      ringColour: hasOutline ? style.outlineColor : style.borderTopColor,
      focusVisible: control.matches(':focus-visible'),
    };
  });
}

/** The value of a CSS variable of the theme, as the browser draws it. */
export async function themeColour(page: Page, variable: string): Promise<string> {
  return page.evaluate((name) => {
    const probe = document.createElement('span');
    probe.style.color = `var(${name})`;
    document.body.append(probe);
    const colour = getComputedStyle(probe).color;
    probe.remove();
    return colour;
  }, variable);
}

/** The colours that a theme gives the focus, as the browser draws them. */
export interface FocusColours {
  /** The outline of a daisyUI field. */
  input: string;
  /** The border of a settings field. */
  field: string;
  /** The outline of a button. */
  button: string;
}

/**
 * Silo draws every focus in its accent. The other themes outline a daisyUI field in the
 * colour of the text, and a settings field and a button in the primary colour.
 */
export async function focusColours(page: Page, look: AuditLook): Promise<FocusColours> {
  if (look.theme === 'silo') {
    const accent = await themeColour(page, '--color-accent');
    return { input: accent, field: accent, button: accent };
  }
  const primary = await themeColour(page, '--color-primary');
  return {
    input: await themeColour(page, '--color-base-content'),
    field: primary,
    button: primary,
  };
}

/**
 * A checked box keeps an edge of about 3:1 against the card; a field or button in focus
 * shows the focus colour of the theme with about 3:1; a primary button stands out from the
 * card with about 3:1.
 */
export function expectEdges(
  look: AuditLook,
  where: string,
  edges: { box?: Edges; field?: [Edges, string]; button?: Edges },
  focus: FocusColours,
) {
  const label = `${where} (${look.id})`;
  if (edges.box) {
    // A checked box shows its fill, or its border when the fill is clear.
    const { fill, border } = edges.box;
    expect.soft(fill ?? border, `the checked box on ${label}`).toBeGreaterThanOrEqual(3);
  }
  if (edges.field) {
    const [field, colour] = edges.field;
    expect.soft(field.focusVisible, `the field in focus on ${label}`).toBe(true);
    expect.soft(field.ringColour, `the focus colour of the field on ${label}`).toBe(colour);
    expect.soft(field.ring, `the focus of the field on ${label}`).toBeGreaterThanOrEqual(3);
  }
  const button = edges.button;
  if (button) {
    expect.soft(button.fill ?? 0, `the primary button on ${label}`).toBeGreaterThanOrEqual(3);
    expect.soft(button.focusVisible, `the primary button in focus on ${label}`).toBe(true);
    expect.soft(button.ringColour, `the focus colour of the button on ${label}`).toBe(focus.button);
    expect.soft(button.ring, `the focus of the button on ${label}`).toBeGreaterThanOrEqual(3);
  }
}

/** Focuses an element as the keyboard does, so that `:focus-visible` matches. */
export async function keyboardFocus(page: Page, control: Locator) {
  await expect(control).toBeVisible();
  await page.keyboard.press('Shift');
  await control.focus();
  // The border of a field fades to the focus colour.
  await waitForAnimations(page);
}

export async function put(page: Page, address: string, data: Record<string, unknown>) {
  const answer = await page.request.put(address, { data });
  expect(answer.ok(), `${address}: ${await answer.text()}`).toBe(true);
}

export interface EffectiveLook {
  layout: 'og' | 'modern';
  theme: string;
  colourSet: string;
  source: 'user' | 'default';
}

export async function readLook(page: Page): Promise<EffectiveLook> {
  const answer = await page.request.get('./api/v2/launch');
  expect(answer.ok(), 'the launch data is read').toBe(true);
  return ((await answer.json()) as { response: { data: { look: EffectiveLook } } }).response.data
    .look;
}

export async function readConfig(page: Page): Promise<Record<string, unknown>> {
  const answer = await page.request.get('./api/v2/config');
  expect(answer.ok(), 'the settings are read').toBe(true);
  return ((await answer.json()) as { response: { data: Record<string, unknown> } }).response.data;
}

/** What puts the look of a person back as it was read. */
export function lookToRestore(look: EffectiveLook): Record<string, unknown> {
  return look.source === 'default'
    ? { layout: null }
    : { layout: look.layout, theme: look.theme, colourSet: look.colourSet };
}

/** The look of the server applies to the page. */
export async function expectLook(page: Page, look: AuditLook) {
  await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
  await expect(page.locator('html')).toHaveAttribute('data-theme', look.id);
}

/** Opens the homepage with the normal answers and waits until every item has loaded. */
export async function openHomepage(page: Page, look: AuditLook) {
  await withHomepageFixtures(page, 'normal');
  await page.goto('./#Homepage');
  await page.reload();
  await expectLook(page, look);
  for (const kind of KINDS) {
    const section = page.locator(`section.org-item-${kind}`);
    await expect(section, `the item ${kind}`).toBeVisible();
  }
  await expect(page.locator('main .loading')).toHaveCount(0);
  for (const kind of KINDS) {
    const section = page.locator(`section.org-item-${kind}`);
    await expect(section.locator('[role="alert"]'), `the item ${kind}`).toHaveCount(0);
    if (kind !== 'customhtml') {
      await expect(section, `the item ${kind}`).toContainText(/\S/);
    }
  }
}

/** Opens a settings page from its address and waits until its content has loaded. */
export async function openSettings(page: Page, address: string, look: AuditLook) {
  await page.goto(`./${address}`);
  await expect(page.locator('.m-page-heading h1')).toBeVisible();
  await expect(page.locator('.org-tab-body [aria-label="Loading"]')).toHaveCount(0);
  await expectLook(page, look);
}

/** A guest in a browser context of its own, so that the Admin stays logged in. */
export interface Guest {
  context: BrowserContext;
  page: Page;
  /** The problems that the browser reported with the page of the guest. */
  problems: string[];
  /** The requests of the guest to another host, which were refused. */
  outside: string[];
}

/**
 * A guest whose page reports problems, refuses every request to another host and gets the
 * wallpaper of the test on its login page.
 */
export async function newGuest(browser: Browser): Promise<Guest> {
  const context = await browser.newContext({ baseURL: `${ORIGIN}/` });
  const page = await context.newPage();
  const problems: string[] = [];
  page.on('pageerror', (error) => problems.push(`error in a script: ${error.message}`));
  page.on('console', (message) => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource')) {
      problems.push(`console: ${message.text()}`);
    }
  });
  const outside: string[] = [];
  await page.route('**/*', async (route: Route) => {
    const request = route.request();
    const address = new URL(request.url());
    if (address.origin === 'https://wallpaper.invalid') {
      await route.fulfill({ status: 200, contentType: 'image/svg+xml', body: WALLPAPER_PICTURE });
      return;
    }
    if (address.origin !== ORIGIN) {
      outside.push(`${request.method()} ${request.url()}`);
      await route.abort('blockedbyclient');
      return;
    }
    if (address.pathname === '/api/v2/launch' && request.method() === 'GET') {
      const answer = await route.fetch();
      const body = (await answer.json()) as {
        response: { data: Record<string, Record<string, unknown>> };
      };
      const appearance = body.response.data.appearance ?? {};
      appearance.loginWallpaper = WALLPAPER;
      appearance.randomMediaImage = '';
      await route.fulfill({ response: answer, body: JSON.stringify(body) });
      return;
    }
    await route.fallback();
  });
  return { context, page, problems, outside };
}

/**
 * Runs the steps of a restore one by one, each even when an earlier one fails, and returns
 * the failures so that the test reports them at the end.
 */
export async function restoreAll(
  steps: [what: string, step: () => Promise<unknown>][],
): Promise<string[]> {
  const failures: string[] = [];
  for (const [what, step] of steps) {
    try {
      await step();
    } catch (error) {
      failures.push(`${what}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return failures;
}
