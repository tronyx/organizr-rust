// The brand themes of the modern layout (Plex, Jellyfin, Emby and Silo, in each of their
// colour sets) in the browser. The other specs of the modern layout check every screen on
// the Organizr theme; this one checks a representative set of screens on every other
// theme and set: the homepage with the items of `homepage-fixtures.ts`, the user menu and
// the look popover of the top bar, the Account page, a settings section with vertical
// tabs, a settings dialog with its accordions open, the phone drawer, and the login page
// of a guest with the Admin default set to the theme. Each text passes the contrast check
// of WCAG AA, and a checked box, a field in focus and a primary button keep edges of about
// 3:1. The font of Silo loads only on Silo. It runs after the walk of `demo.spec.ts`, on
// the same install. No request leaves the machine: the fixtures answer the items, the
// wallpaper is under `.invalid` and answered here, and every other request to another host
// is refused. The test leaves the settings and the look of the install as it found them.

import type { Locator, Page, Route } from '@playwright/test';

import { MODERN_THEMES } from '../src/lib/look';
import { auditContrast, describeLowContrast } from './contrast';
import { expect, test } from './fixtures';
import {
  allItemSettings,
  COMBINE_SWITCHES,
  HOMEPAGE_FIXTURES,
  unroutedRequests,
  withHomepageFixtures,
} from './homepage-fixtures';
import { logIn } from './login';
import { ADDRESS } from './server';

const ORIGIN = `http://${ADDRESS}`;

/**
 * Every theme and colour set but those of Organizr, which the other specs check, and those
 * of the scene themes, whose audit must also know the scene behind the page.
 */
const LOOKS = MODERN_THEMES.filter(
  (theme) => theme.id !== 'organizr' && theme.scene === undefined,
).flatMap((theme) =>
  theme.colourSets.map((set) => ({
    theme: theme.id,
    set: set.id,
    setLabel: set.label,
    id: `m-${theme.id}-${set.id}`,
  })),
);
type Look = (typeof LOOKS)[number];

/** The kinds that show on the homepage when every item is on and none is combined. */
const KINDS = HOMEPAGE_FIXTURES.map((fixture) => fixture.kind);

/** Every setting that the test changes. */
const CHANGED = [...Object.keys(allItemSettings()), ...Object.values(COMBINE_SWITCHES)];

/** The wallpaper of the login page: dark, so the card shows its own colours. */
const WALLPAPER = 'https://wallpaper.invalid/dark.svg';
const WALLPAPER_PICTURE =
  '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#050505"/></svg>';

/** The fades of the page and of its dialogs are over. A spinner turns for ever. */
async function waitForAnimations(page: Page) {
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
async function checkContrast(page: Page, look: Look, where: string, within?: string) {
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
async function expectNoSideScroll(page: Page, where: string) {
  const scroll = await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    return root.scrollWidth - root.clientWidth;
  });
  expect(scroll, `${where} scrolls sideways`).toBe(0);
}

/** Every control of the top bar ends inside the window (the bar is `position: fixed`). */
async function expectBarFits(page: Page, where: string) {
  const bar = await page.evaluate(() => {
    const header = document.querySelector('header.org-header');
    const controls = [...(header?.querySelectorAll('button, a, img') ?? [])]
      .map((element) => {
        const { left, right, width } = element.getBoundingClientRect();
        return { name: element.getAttribute('aria-label') ?? element.tagName, left, right, width };
      })
      .filter((control) => control.width > 0);
    return { width: window.innerWidth, controls };
  });
  expect(bar.controls.length, `controls of the top bar ${where}`).toBeGreaterThan(0);
  for (const control of bar.controls) {
    expect.soft(control.left, `${control.name} ${where}: left edge`).toBeGreaterThanOrEqual(0);
    expect
      .soft(control.right, `${control.name} ${where}: right edge`)
      .toBeLessThanOrEqual(bar.width);
  }
}

/** What a control shows of itself, as contrast ratios against what is behind it. */
interface Edges {
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
async function edgesOf(control: Locator): Promise<Edges> {
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
async function themeColour(page: Page, variable: string): Promise<string> {
  return page.evaluate((name) => {
    const probe = document.createElement('span');
    probe.style.color = `var(${name})`;
    document.body.append(probe);
    const colour = getComputedStyle(probe).color;
    probe.remove();
    return colour;
  }, variable);
}

/**
 * The font of Silo: the family of the page, and whether its face was loaded. On every other
 * theme the browser never fetches it.
 */
async function siloFont(page: Page) {
  return page.evaluate(async () => {
    await document.fonts.ready;
    const faces = [...document.fonts].filter((face) =>
      face.family.replace(/['"]/g, '').startsWith('Plus Jakarta Sans'),
    );
    return {
      family: getComputedStyle(document.documentElement).fontFamily,
      faces: faces.length,
      loaded: faces.filter((face) => face.status === 'loaded').length,
    };
  });
}

/**
 * The links in running text that look like the text around them: no underline and less
 * than 3:1 against its colour. Buttons, menus and the top bar are not running text.
 */
async function hiddenLinks(page: Page) {
  return page.evaluate(() => {
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const rgb = (colour: string) => {
      if (!context) return [0, 0, 0];
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = colour;
      context.fillRect(0, 0, 1, 1);
      return [...context.getImageData(0, 0, 1, 1).data].slice(0, 3);
    };
    const luminance = (colour: number[]) => {
      const channel = (value: number) => {
        const c = value / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      };
      const [r = 0, g = 0, b = 0] = colour;
      return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    };
    const found: string[] = [];
    let inText = 0;
    let alone = 0;
    for (const link of document.querySelectorAll('a[href]')) {
      if (
        link.closest('header, nav, [role="menu"], .btn, .m-settings-side') ||
        link.matches('.btn, .link') ||
        !link.checkVisibility() ||
        !link.textContent?.trim()
      ) {
        continue;
      }
      const parent = link.parentElement;
      const besides = [...(parent?.childNodes ?? [])].some(
        (node) => node.nodeType === Node.TEXT_NODE && (node.textContent ?? '').trim() !== '',
      );
      if (!parent || !besides) {
        alone += 1;
        continue;
      }
      inText += 1;
      const style = getComputedStyle(link);
      const [one, two] = [
        luminance(rgb(style.color)),
        luminance(rgb(getComputedStyle(parent).color)),
      ].sort((a, b) => b - a) as [number, number];
      const ratio = (one + 0.05) / (two + 0.05);
      if (!style.textDecorationLine.includes('underline') && ratio < 3) {
        found.push(`"${link.textContent.trim().slice(0, 40)}" (${ratio.toFixed(2)}:1)`);
      }
    }
    return { found, inText, alone };
  });
}

/** The background and text colour of a control, as sRGB. */
async function fillOf(control: Locator) {
  return control.evaluate((element) => {
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const rgb = (colour: string) => {
      if (!context) return [0, 0, 0];
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = colour;
      context.fillRect(0, 0, 1, 1);
      return [...context.getImageData(0, 0, 1, 1).data].slice(0, 3);
    };
    const style = getComputedStyle(element);
    return {
      background: style.backgroundColor,
      rgb: rgb(style.backgroundColor),
      text: rgb(style.color),
    };
  });
}

/** The contrast ratio of two sRGB colours. */
function ratioOf(one: number[], two: number[]): number {
  const luminance = (colour: number[]) => {
    const channel = (value: number) => {
      const c = value / 255;
      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };
    const [r = 0, g = 0, b = 0] = colour;
    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
  };
  const [high, low] = [luminance(one), luminance(two)].sort((a, b) => b - a) as [number, number];
  return (high + 0.05) / (low + 0.05);
}

async function put(page: Page, address: string, data: Record<string, unknown>) {
  const answer = await page.request.put(address, { data });
  expect(answer.ok(), `${address}: ${await answer.text()}`).toBe(true);
}

interface EffectiveLook {
  layout: 'og' | 'modern';
  theme: string;
  colourSet: string;
  source: 'user' | 'default';
}

async function readLook(page: Page): Promise<EffectiveLook> {
  const answer = await page.request.get('./api/v2/launch');
  expect(answer.ok(), 'the launch data is read').toBe(true);
  return ((await answer.json()) as { response: { data: { look: EffectiveLook } } }).response.data
    .look;
}

async function readConfig(page: Page): Promise<Record<string, unknown>> {
  const answer = await page.request.get('./api/v2/config');
  expect(answer.ok(), 'the settings are read').toBe(true);
  return ((await answer.json()) as { response: { data: Record<string, unknown> } }).response.data;
}

/** The look of the server applies to the page. */
async function expectLook(page: Page, look: Look) {
  await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
  await expect(page.locator('html')).toHaveAttribute('data-theme', look.id);
}

/** Opens the homepage with the normal answers and waits until every item has loaded. */
async function openHomepage(page: Page, look: Look) {
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
async function openSettings(page: Page, address: string, look: Look) {
  await page.goto(`./${address}`);
  await expect(page.locator('.m-page-heading h1')).toBeVisible();
  await expect(page.locator('.org-tab-body [aria-label="Loading"]')).toHaveCount(0);
  await expectLook(page, look);
}

/** The colours that a theme gives the focus, as the browser draws them. */
interface FocusColours {
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
async function focusColours(page: Page, look: Look): Promise<FocusColours> {
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
function expectEdges(
  look: Look,
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
async function keyboardFocus(page: Page, control: Locator) {
  await expect(control).toBeVisible();
  await page.keyboard.press('Shift');
  await control.focus();
  // The border of a field fades to the focus colour.
  await waitForAnimations(page);
}

test('the_brand_themes_of_the_modern_layout_pass_the_audit', async ({ page, browser }) => {
  test.setTimeout(300_000);
  expect(LOOKS.length, 'theme and colour sets to check').toBe(9);
  await logIn(page);
  const lookBefore = await readLook(page);
  const config = await readConfig(page);
  const defaultsBefore = {
    layout: config.layout,
    modernTheme: config.modernTheme,
    modernColourSet: config.modernColourSet,
  };
  const itemsBefore = Object.fromEntries(CHANGED.map((key) => [key, config[key] ?? '']));
  // The sessions of Overseerr and Ombi, which the search of the requests needs.
  await page
    .context()
    .addCookies(
      ['connect_sid', 'Auth'].map((name) => ({ name, value: 'fixture', url: `${ORIGIN}/` })),
    );

  // A guest in a context of its own, so that the Admin stays logged in. Its page reports
  // problems, refuses every request to another host and gets the wallpaper of the test.
  const guestContext = await browser.newContext({ baseURL: `${ORIGIN}/` });
  const guest = await guestContext.newPage();
  const guestProblems: string[] = [];
  guest.on('pageerror', (error) => guestProblems.push(`error in a script: ${error.message}`));
  guest.on('console', (message) => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource')) {
      guestProblems.push(`console: ${message.text()}`);
    }
  });
  const outside: string[] = [];
  await guest.route('**/*', async (route: Route) => {
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

  const linkNotes: string[] = [];
  const hoverNotes: string[] = [];
  try {
    await put(page, './api/v2/config', {
      ...allItemSettings(),
      ...Object.fromEntries(Object.values(COMBINE_SWITCHES).map((key) => [key, false])),
    });

    for (const look of LOOKS) {
      await put(page, './api/v2/user/look', {
        layout: 'modern',
        theme: look.theme,
        colourSet: look.set,
      });
      await put(page, './api/v2/config', {
        layout: 'modern',
        modernTheme: look.theme,
        modernColourSet: look.set,
      });
      await openHomepage(page, look);
      const focus = await focusColours(page, look);

      // The homepage, and the font of the theme.
      await checkContrast(page, look, 'the homepage');
      const font = await siloFont(page);
      if (look.theme === 'silo') {
        expect(font.family, `the font on ${look.id}`).toMatch(/^"?Plus Jakarta Sans Variable/);
        expect(font.loaded, `faces of the Silo font loaded on ${look.id}`).toBeGreaterThan(0);
      } else {
        expect(font.family, `the font on ${look.id}`).not.toContain('Plus Jakarta');
        expect(font.loaded, `faces of the Silo font loaded on ${look.id}`).toBe(0);
      }
      if (look.theme === 'silo') {
        const links = await hiddenLinks(page);
        linkNotes.push(
          `the homepage (${look.id}): ${links.inText} in text, ${links.alone} alone, hidden: ${links.found.join(', ') || 'none'}`,
        );
        expect.soft(links.found, `links that look like text on ${look.id}`).toEqual([]);
      }

      // The user menu and the look popover of the top bar.
      await page.getByRole('button', { name: 'Menu of the user' }).click();
      await expect(page.getByRole('menuitem', { name: 'Account Settings' })).toBeVisible();
      await checkContrast(page, look, 'the user menu', '[role="menu"]');
      await page.keyboard.press('Escape');
      await expect(page.locator('[role="menu"]')).toHaveCount(0);
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

      // A settings section with vertical tabs, each tab, and a field in focus.
      await openSettings(page, '#/settings/general', look);
      const tabs = page.locator('.org-vtabs [role="tab"]');
      const count = await tabs.count();
      expect(count, 'tabs of the section Main').toBeGreaterThan(1);
      for (let index = 0; index < count; index += 1) {
        const tab = tabs.nth(index);
        const name = ((await tab.textContent()) ?? '').trim();
        await tab.click();
        await expect(tab).toHaveAttribute('aria-selected', 'true');
        await checkContrast(page, look, `the section Main, tab "${name}"`);
      }
      await tabs.first().click();
      const field = page.getByRole('tabpanel').getByRole('textbox').first();
      await keyboardFocus(page, field);
      expectEdges(look, 'the section Main', { field: [await edgesOf(field), focus.field] }, focus);
      if (look.theme === 'silo') {
        const links = await hiddenLinks(page);
        linkNotes.push(
          `the section Main (${look.id}): ${links.inText} in text, ${links.alone} alone, hidden: ${links.found.join(', ') || 'none'}`,
        );
        expect.soft(links.found, `links that look like text on ${look.id}`).toEqual([]);
      }

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

      // A button of the form shows its hover: on Silo the off-white darkens.
      await mailer.getByRole('tab', { name: 'Test & Options' }).click();
      const send = mailer.locator('.org-form-button', { hasText: 'Send' });
      const rest = await fillOf(send);
      await send.hover();
      await expect.poll(async () => (await fillOf(send)).background).not.toBe(rest.background);
      // The fill fades to the hover colour.
      await waitForAnimations(page);
      const hovered = await fillOf(send);
      const apart = ratioOf(rest.rgb, hovered.rgb);
      const text = ratioOf(hovered.rgb, hovered.text);
      hoverNotes.push(
        `${look.id}: ${rest.background} -> ${hovered.background}, ${apart.toFixed(2)}:1 apart, text ${text.toFixed(2)}:1`,
      );
      if (look.theme === 'silo') {
        expect.soft(apart, `the hover of a form button on ${look.id}`).toBeGreaterThanOrEqual(1.3);
      }
      expect
        .soft(text, `the text of a hovered form button on ${look.id}`)
        .toBeGreaterThanOrEqual(4.5);
      await page.mouse.move(0, 0);
      await page.keyboard.press('Escape');
      await expect(mailer).toBeHidden();

      // On a phone: the drawer of the menu, and nothing scrolls sideways.
      await page.setViewportSize({ width: 375, height: 812 });
      try {
        await page.goto('./#Homepage');
        await expect(page.locator('section.org-item-calendar')).toBeVisible();
        await expectNoSideScroll(page, `the homepage on a phone (${look.id})`);
        await expectBarFits(page, `on a phone (${look.id})`);
        await page.getByRole('button', { name: 'Show the menu of tabs' }).click();
        const drawer = page.getByRole('dialog');
        await expect(drawer.getByRole('navigation', { name: 'Tabs' })).toBeVisible();
        await checkContrast(page, look, 'the drawer on a phone', '[role="dialog"]');
        await page.keyboard.press('Escape');
        await expect(drawer).toBeHidden();
      } finally {
        await page.setViewportSize({ width: 1280, height: 720 });
      }

      // The login page of a guest, in the Admin default: its card, a checked box, a
      // field in focus and the primary button in focus.
      await guest.goto('./#/login');
      await guest.reload();
      await expect(guest.getByRole('heading', { name: 'Log in', exact: true })).toBeAttached();
      await expectLook(guest, look);
      await expect(guest.locator('main.org-login-register')).toHaveCSS(
        'background-image',
        `url("${WALLPAPER}")`,
      );
      await checkContrast(guest, look, 'the login page of a guest');
      await guest.getByRole('checkbox', { name: 'Stay logged in on this device' }).check();
      const box = await edgesOf(guest.locator('main input.checkbox'));
      const username = guest.locator('#login-username');
      await keyboardFocus(guest, username);
      const typed = await edgesOf(username);
      const submit = guest.locator('main button.btn-primary');
      await keyboardFocus(guest, submit);
      expectEdges(
        look,
        'the login page of a guest',
        { box, field: [typed, focus.input], button: await edgesOf(submit) },
        focus,
      );
      if (look.theme === 'silo') {
        const links = await hiddenLinks(guest);
        linkNotes.push(
          `the login page (${look.id}): ${links.inText} in text, ${links.alone} alone, hidden: ${links.found.join(', ') || 'none'}`,
        );
        expect.soft(links.found, `links that look like text on ${look.id}`).toEqual([]);
      }
    }
    expect(unroutedRequests(page), 'requests that no fixture answered').toEqual([]);
  } finally {
    // Each step runs even when an earlier one fails; the failures are reported at the end.
    const failures: string[] = [];
    const restore = async (what: string, step: () => Promise<unknown>) => {
      try {
        await step();
      } catch (error) {
        failures.push(`${what}: ${error instanceof Error ? error.message : String(error)}`);
      }
    };
    await restore('the guest', () => guestContext.close());
    await restore('the routes', () => page.unrouteAll({ behavior: 'ignoreErrors' }));
    await restore('the viewport', () => page.setViewportSize({ width: 1280, height: 720 }));
    await restore('the items', () => put(page, './api/v2/config', itemsBefore));
    await restore('the Admin defaults', () => put(page, './api/v2/config', defaultsBefore));
    await restore('the look of the Admin', () =>
      put(
        page,
        './api/v2/user/look',
        lookBefore.source === 'default'
          ? { layout: null }
          : { layout: lookBefore.layout, theme: lookBefore.theme, colourSet: lookBefore.colourSet },
      ),
    );
    console.log(`Plain links on Silo:\n  ${linkNotes.join('\n  ')}`);
    console.log(`The hover of a form button:\n  ${hoverNotes.join('\n  ')}`);
    expect.soft(failures, 'the restore of the install').toEqual([]);
  }
  expect(outside, 'requests of the guest to another host').toEqual([]);
  expect(guestProblems, 'the browser reported problems with the page of the guest').toEqual([]);
});
