// The homepage on the modern layout: every item, with the answers of `homepage-fixtures.ts`
// (normal, empty and failed), passes the contrast check of WCAG AA in the dark colour set
// and in the light one (which people cannot pick yet, but whose styles exist), in each of
// its tabs, menus and dialogs. No request leaves the machine: every item address is
// under `.invalid`, and the fixtures answer every route of the items. The tests leave the
// settings and the look of the install as they found them.

import type { Page } from '@playwright/test';

import { auditContrast, describeLowContrast } from './contrast';
import { expect, test } from './fixtures';
import {
  allItemSettings,
  COMBINE_SWITCHES,
  HOMEPAGE_FIXTURES,
  timesAnswered,
  unroutedRequests,
  type Variant,
  withHomepageFixtures,
} from './homepage-fixtures';
import { logIn } from './login';
import { ADDRESS } from './server';

type ColourSet = 'dark' | 'light';
const SETS: ColourSet[] = ['dark', 'light'];

/** The theme and colour set of the page, as the look of the person would set them. */
async function useColourSet(page: Page, set: ColourSet) {
  await page.evaluate((wanted) => {
    document.documentElement.dataset.theme = `m-organizr-${wanted}`;
    document.documentElement.dataset.colourSet = wanted;
  }, set);
  await expect(page.locator('html')).toHaveAttribute('data-theme', `m-organizr-${set}`);
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

/** The routes that an item asks when it shows: not the searches, details or actions. */
const LOAD_PATTERNS = HOMEPAGE_FIXTURES.flatMap((fixture) =>
  fixture.routes
    .map((route) => route.pattern)
    .filter((pattern) => !pattern.includes(' ') && !pattern.includes('*'))
    .filter((pattern) => pattern !== 'homepage/donate'),
);

/** The kinds that show on the homepage when every item is on and none is combined. */
const KINDS = HOMEPAGE_FIXTURES.map((fixture) => fixture.kind);

/** Every setting that the tests change. */
const CHANGED = [...Object.keys(allItemSettings()), ...Object.values(COMBINE_SWITCHES)];

const look = (page: Page, layout: 'og' | 'modern') =>
  page.request.put('./api/v2/user/look', {
    data: { layout, theme: layout === 'og' ? 'Organizr' : 'organizr', colourSet: 'dark' },
  });

async function changeSettings(page: Page, values: Record<string, unknown>) {
  const answer = await page.request.put('./api/v2/config', { data: values });
  expect(answer.ok(), `the settings are changed: ${await answer.text()}`).toBe(true);
}

/**
 * Switches every item on and the look to modern, runs `body`, and puts the settings and
 * the look back. A secret that was set comes back as the text the API shows for it, which
 * the server leaves as it is.
 */
async function withEveryItem(page: Page, body: () => Promise<void>) {
  await logIn(page);
  const answer = await page.request.get('./api/v2/config');
  expect(answer.ok()).toBe(true);
  const all = ((await answer.json()) as { response: { data: Record<string, unknown> } }).response
    .data;
  const original = Object.fromEntries(CHANGED.map((key) => [key, all[key] ?? '']));
  // The sessions of Overseerr and Ombi, which the search of the requests needs.
  await page.context().addCookies(
    ['connect_sid', 'Auth'].map((name) => ({
      name,
      value: 'fixture',
      url: `http://${ADDRESS}/`,
    })),
  );
  try {
    await changeSettings(page, {
      ...allItemSettings(),
      ...Object.fromEntries(Object.values(COMBINE_SWITCHES).map((key) => [key, false])),
    });
    expect((await look(page, 'modern')).ok()).toBe(true);
    await body();
    expect(unroutedRequests(page), 'requests that no fixture answered').toEqual([]);
  } finally {
    await page.unrouteAll({ behavior: 'ignoreErrors' });
    await changeSettings(page, original);
    expect((await look(page, 'og')).ok()).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
  }
}

/** Opens the homepage with the answers of `variant` and waits until every item has loaded. */
async function openHomepage(page: Page, variant: Variant, kinds: string[] = KINDS) {
  await withHomepageFixtures(page, variant);
  await page.goto('./#Homepage');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-layout', 'modern');
  for (const kind of kinds) {
    await expect(page.locator(`section.org-item-${kind}`), `the item ${kind}`).toHaveCount(1);
  }
  const patterns = LOAD_PATTERNS.filter((pattern) =>
    HOMEPAGE_FIXTURES.some(
      (fixture) =>
        kinds.some((kind) => kind === fixture.kind || kind === 'downloader') &&
        fixture.routes.some((route) => route.pattern === pattern),
    ),
  );
  await expect
    .poll(() => patterns.filter((pattern) => timesAnswered(page, pattern) === 0), {
      message: 'routes that were not asked yet',
    })
    .toEqual([]);
  await expect(page.locator('main .loading')).toHaveCount(0);
  if (variant === 'normal') {
    for (const kind of kinds) {
      const section = page.locator(`section.org-item-${kind}`);
      await expect(section, `the item ${kind}`).toBeVisible();
      await expect(section.locator('[role="alert"]'), `the item ${kind}`).toHaveCount(0);
      if (kind === 'customhtml') {
        await expect(section.locator('iframe')).toBeVisible();
      } else {
        await expect(section, `the item ${kind}`).toContainText(/\S/);
      }
    }
  }
}

/** Closes the dialog on top, with the Escape key. */
async function closeDialog(page: Page) {
  const open = await page.locator('dialog[open]').count();
  await page.keyboard.press('Escape');
  await expect(page.locator('dialog[open]')).toHaveCount(open - 1);
}

/** Opens each tab, menu and dialog of the items, and checks each of them. */
async function checkStates(page: Page, set: ColourSet) {
  const item = (kind: string) => page.locator(`section.org-item-${kind}`);

  // The history of the queues of SABnzbd and NZBGet.
  for (const kind of ['sabnzbd', 'nzbget']) {
    await item(kind).getByRole('tab', { name: 'History' }).click();
    await checkContrast(page, `the history of ${kind} (${set})`, `section.org-item-${kind}`);
    await item(kind).getByRole('tab', { name: 'Queue' }).click();
  }

  // The menus of filters over the rows of posters.
  const filters = page.locator('main button[aria-label^="Filter "][aria-expanded="false"]');
  const count = await filters.count();
  expect(count, 'rows with a filter menu').toBeGreaterThan(0);
  for (let index = 0; index < count; index += 1) {
    await filters.first().click();
  }
  await expect(page.locator('main .org-row-menu')).toHaveCount(count);
  await checkContrast(page, `the filter menus of the rows (${set})`);
  const opened = page.locator('main button[aria-label^="Filter "][aria-expanded="true"]');
  for (let index = 0; index < count; index += 1) {
    await opened.first().click();
  }

  // The calendar: its filters, each view, and the details of an event.
  const calendar = item('calendar');
  await calendar.getByRole('button', { name: 'Filter' }).click();
  await checkContrast(page, `the filters of the calendar (${set})`, 'section.org-item-calendar');
  await calendar.getByRole('button', { name: 'Filter' }).click();
  for (const view of ['Week', 'Day', 'List', 'Month']) {
    await calendar.getByRole('button', { name: view, exact: true }).click();
    await checkContrast(page, `the calendar, ${view} (${set})`, 'section.org-item-calendar');
  }
  await calendar.getByRole('button', { name: /Missing Film/ }).click();
  await expect(page.locator('dialog[open]')).toHaveCount(1);
  await checkContrast(page, `the details of an event (${set})`);
  await closeDialog(page);

  // A stream: how it plays, and its details.
  const streams = item('plexstreams');
  await streams.getByRole('button', { name: 'How The Show - The One plays' }).click();
  await expect(page.locator('dialog[open]')).toHaveCount(1);
  await checkContrast(page, `how a stream plays (${set})`);
  await closeDialog(page);
  await streams.getByRole('button', { name: 'Details of The Show' }).click();
  await expect(page.locator('dialog[open]')).toContainText('A long plot.');
  await checkContrast(page, `the details of a stream (${set})`);
  await closeDialog(page);

  // The details of a new item of each media server, and of JellyStat.
  for (const kind of ['plexrecent', 'embyrecent', 'jellyfinrecent', 'jellystat']) {
    await item(kind).locator('.org-poster button').first().click();
    await expect(page.locator('dialog[open]')).toContainText(/A long plot\.|A film\./);
    await checkContrast(page, `the details of a poster of ${kind} (${set})`);
    await closeDialog(page);
  }

  // The other playlist.
  await item('plexplaylists').getByLabel('Playlist').selectOption('Kids');
  await expect(item('plexplaylists').getByRole('list', { name: 'Kids' })).toBeVisible();
  await checkContrast(page, `the playlist Kids (${set})`, 'section.org-item-plexplaylists');
  await item('plexplaylists').getByLabel('Playlist').selectOption('Filmnight');

  // A request, and the search of the requests with its results and one of them.
  for (const kind of ['overseerr', 'ombi']) {
    await item(kind).locator('.org-poster button').first().click();
    await expect(page.locator('dialog[open]')).toHaveCount(1);
    await checkContrast(page, `a request of ${kind} (${set})`);
    await closeDialog(page);

    await item(kind).getByRole('button', { name: 'Request a Show or Movie' }).click();
    const search = page.getByRole('dialog', { name: 'Request a Show or Movie' });
    await expect(search).toBeVisible();
    await checkContrast(page, `the search of ${kind} (${set})`);
    await search.getByRole('searchbox').fill('The F');
    await search.getByRole('button', { name: 'Search' }).click();
    await expect(search.getByRole('list', { name: /Results in/ })).toBeVisible();
    await checkContrast(page, `the results of ${kind} (${set})`);
    await search.getByRole('button', { name: /The Film/ }).click();
    await expect(page.locator('dialog[open]')).toHaveCount(2);
    await checkContrast(page, `a result of ${kind} (${set})`);
    await closeDialog(page);
    await closeDialog(page);
  }

  // The search of the indexers.
  for (const [kind, name] of [
    ['jackett', 'Jackett'],
    ['prowlarr', 'Prowlarr'],
  ] as const) {
    await item(kind).getByRole('searchbox').fill('A Film');
    await item(kind).getByRole('button', { name: 'Search' }).click();
    await expect(
      item(kind).getByRole('table', { name: `Releases that ${name} found` }),
    ).toBeVisible();
    await checkContrast(page, `the releases of ${kind} (${set})`, `section.org-item-${kind}`);
    await item(kind).getByRole('button', { name: 'Clear the search' }).click();
  }

  // The donations of the person.
  await item('donate').getByRole('button', { name: 'Show my donations' }).click();
  await expect(item('donate').getByRole('table', { name: 'Donations' })).toBeVisible();
  await checkContrast(page, `the donations (${set})`, 'section.org-item-donate');
  await item('donate').getByRole('button', { name: 'Hide my donations' }).click();
}

/** Opens what a failed or empty answer shows only after a click: searches and lists. */
async function checkOtherStates(page: Page, variant: Variant, set: ColourSet) {
  const item = (kind: string) => page.locator(`section.org-item-${kind}`);
  for (const kind of ['jackett', 'prowlarr']) {
    await item(kind).getByRole('searchbox').fill('A Film');
    await item(kind).getByRole('button', { name: 'Search' }).click();
    // Nothing found keeps the button that clears the search; a failure shows its reason.
    const clear = item(kind).getByRole('button', { name: 'Clear the search' });
    await expect(variant === 'empty' ? clear : item(kind).getByRole('alert')).toBeVisible();
    await checkContrast(
      page,
      `the releases of ${kind} (${variant}, ${set})`,
      `section.org-item-${kind}`,
    );
    if (variant === 'empty') {
      await clear.click();
    }
  }
  const donate = item('donate');
  await donate.getByRole('button', { name: 'Show my donations' }).click();
  // An empty list says so, and a failure shows its reason.
  await expect(
    variant === 'empty' ? donate.getByText('No donations yet.') : donate.getByRole('alert'),
  ).toBeVisible();
  await checkContrast(page, `the donations (${variant}, ${set})`, 'section.org-item-donate');
  const hide = donate.getByRole('button', { name: 'Hide my donations' });
  if (await hide.isVisible()) {
    await hide.click();
  }
}

test('homepage_items_pass_contrast_on_modern_dark_and_light', async ({ page }) => {
  // One run takes about 25 seconds.
  test.setTimeout(75_000);
  await withEveryItem(page, async () => {
    for (const variant of ['normal', 'empty', 'error'] as const) {
      await openHomepage(page, variant);
      for (const set of SETS) {
        await useColourSet(page, set);
        await checkContrast(page, `the homepage (${variant}, ${set})`);
        if (variant === 'normal') {
          await checkStates(page, set);
        } else {
          await checkOtherStates(page, variant, set);
        }
      }
    }

    // The downloaders with their combine switch share one item with a tab for each.
    await changeSettings(
      page,
      Object.fromEntries(Object.values(COMBINE_SWITCHES).map((key) => [key, true])),
    );
    const members = Object.keys(COMBINE_SWITCHES);
    const others = KINDS.filter((kind) => !members.includes(kind));
    await openHomepage(page, 'normal', [...others, 'downloader']);
    for (const member of members) {
      await expect(page.locator(`section.org-item-${member}`)).toHaveCount(0);
    }
    const tabs = page.locator('section.org-item-downloader').getByRole('tab');
    await expect(tabs).toHaveCount(members.length);
    for (const set of SETS) {
      await useColourSet(page, set);
      for (let index = 0; index < members.length; index += 1) {
        await tabs.nth(index).click();
        await expect(tabs.nth(index)).toHaveAttribute('aria-selected', 'true');
        const name = (await tabs.nth(index).getAttribute('aria-label')) ?? '';
        await checkContrast(
          page,
          `the downloader tab ${name} (${set})`,
          'section.org-item-downloader',
        );
      }
    }
    await changeSettings(
      page,
      Object.fromEntries(Object.values(COMBINE_SWITCHES).map((key) => [key, false])),
    );

    // On a phone the items stack, and only tables and rows scroll, inside their item.
    await page.setViewportSize({ width: 375, height: 812 });
    await openHomepage(page, 'normal');
    const overflow = await page.evaluate(() => {
      const root = document.scrollingElement ?? document.documentElement;
      const wide = [...document.querySelectorAll('main > section')]
        .filter((section) => section.getBoundingClientRect().right > window.innerWidth + 1)
        .map((section) => section.className);
      return { scroll: root.scrollWidth - root.clientWidth, wide };
    });
    expect(overflow, 'the page scrolls sideways on a phone').toEqual({ scroll: 0, wide: [] });
    for (const set of SETS) {
      await useColourSet(page, set);
      await checkContrast(page, `the homepage on a phone (${set})`);
    }
  });
});

test('text_over_pictures_has_a_scrim', async ({ page }, testInfo) => {
  test.setTimeout(30_000);
  await withEveryItem(page, async () => {
    await openHomepage(page, 'normal');
    for (const set of SETS) {
      await useColourSet(page, set);
      // A text whose box is over a picture must have a background of its own (or of an
      // element between it and the picture) that hides most of the picture.
      const bare = await page.evaluate(() => {
        const canvas = document.createElement('canvas');
        const context = canvas.getContext('2d', { willReadFrequently: true });
        const alphaOf = (colour: string) => {
          if (!context) return 0;
          context.clearRect(0, 0, 1, 1);
          context.fillStyle = 'rgba(0, 0, 0, 0)';
          context.fillStyle = colour;
          context.fillRect(0, 0, 1, 1);
          return (context.getImageData(0, 0, 1, 1).data[3] ?? 0) / 255;
        };
        const pictures = [...document.querySelectorAll('main img')].filter((img) => {
          const box = img.getBoundingClientRect();
          return box.width > 48 && box.height > 48 && img.checkVisibility();
        });
        const found: string[] = [];
        for (const element of document.querySelectorAll('main *')) {
          const text = [...element.childNodes]
            .filter((node) => node.nodeType === Node.TEXT_NODE)
            .map((node) => node.textContent ?? '')
            .join('')
            .trim();
          if (!text || !element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) {
            continue;
          }
          const box = element.getBoundingClientRect();
          if (box.width <= 1 || box.height <= 1) {
            continue;
          }
          for (const picture of pictures) {
            const area = picture.getBoundingClientRect();
            const overlaps =
              box.left < area.right - 1 &&
              box.right > area.left + 1 &&
              box.top < area.bottom - 1 &&
              box.bottom > area.top + 1;
            if (!overlaps) {
              continue;
            }
            let scrim = false;
            for (
              let current: Element | null = element;
              current && !current.contains(picture);
              current = current.parentElement
            ) {
              const style = getComputedStyle(current);
              if (alphaOf(style.backgroundColor) * Number(style.opacity) >= 0.6) {
                scrim = true;
                break;
              }
            }
            if (!scrim) {
              found.push(`"${text.slice(0, 40)}" in ${element.closest('section')?.className}`);
            }
          }
        }
        return { found, pictures: pictures.length };
      });
      expect(bare.pictures, `pictures on the homepage (${set})`).toBeGreaterThan(10);
      expect.soft(bare.found, `texts over a picture without a scrim (${set})`).toEqual([]);
      for (const kind of ['plexstreams', 'plexrecent', 'overseerr', 'tautulli']) {
        await testInfo.attach(`${kind}-${set}.png`, {
          body: await page.locator(`section.org-item-${kind}`).screenshot(),
          contentType: 'image/png',
        });
      }
    }
  });
});

test('calendar_events_follow_custom_css_on_modern', async ({ page }) => {
  test.setTimeout(30_000);
  await withEveryItem(page, async () => {
    const answer = await page.request.get('./api/v2/config');
    const before = ((await answer.json()) as { response: { data: Record<string, unknown> } })
      .response.data.customCss;
    try {
      await changeSettings(page, { customCss: ':root { --org-event-missing: rgb(1, 2, 3); }' });
      await openHomepage(page, 'normal', ['calendar']);
      for (const set of SETS) {
        await useColourSet(page, set);
        await expect(
          page.locator('section.org-item-calendar .org-event-missing').first(),
        ).toHaveCSS('color', 'rgb(1, 2, 3)');
      }
    } finally {
      await changeSettings(page, { customCss: before ?? '' });
    }
  });
});
