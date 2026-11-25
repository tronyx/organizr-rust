import { cleanup, render, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { loadModernStyles } from './lib/modernStyles';
import { session } from './lib/session.svelte';
import { stubApi } from './test-helpers';

// The chunk of the modern shell does not load, as after an update with a cached page.
vi.mock('./pages/ModernShell.svelte', () => {
  throw new Error('Failed to fetch dynamically imported module');
});

// The modern style sheet loads when a test says so. `modernStyles.test.ts` tests the link.
const styles = vi.hoisted(() => ({ loaded: () => {} }));
vi.mock('./lib/modernStyles', () => ({
  loadModernStyles: vi.fn(
    () =>
      new Promise<void>((resolve) => {
        styles.loaded = resolve;
      }),
  ),
}));

/** The answer of `GET launch` for the layout `layout`, with `status` as the install status. */
function launch(layout: 'og' | 'modern', status = 'ok') {
  return {
    data: {
      user: { username: 'guest', group: 'Guest', groupID: 999, loggedin: false },
      look: { layout, theme: 'organizr', colorSet: 'dark', source: 'user' },
      tabs: [],
      categories: [],
      plugins: { includes: {} },
      appearance: { title: 'My Organizr' },
      settings: {
        sso: { plex: { oAuthEnabled: false } },
        homepage: { refresh: {}, options: {} },
        misc: {},
        ping: {},
      },
      status: { status },
    },
  };
}

afterEach(() => {
  cleanup();
  session.launch = null;
  delete document.documentElement.dataset.layout;
});

// The test imports the whole app cold, which takes about 1.5 seconds alone and more when
// the test run is busy: the test and its wait get room for that.
describe('App', () => {
  it('falls_back_to_the_original_shell_when_the_modern_chunk_does_not_load', async () => {
    stubApi({ 'GET launch': launch('modern') });
    const { default: App } = await import('./App.svelte');

    const { container } = render(App);
    await waitFor(() => expect(loadModernStyles).toHaveBeenCalled(), { timeout: 10_000 });
    styles.loaded();

    // The original shell has the class `org-with-menu` on its bar.
    await waitFor(
      () => expect(container.ownerDocument.querySelector('.org-header')).not.toBeNull(),
      { timeout: 10_000 },
    );
    expect(document.querySelector('.org-header.org-with-menu')).not.toBeNull();
    expect(document.querySelector('.loading-spinner')).toBeNull();
  }, 30_000);

  it('og_does_not_request_the_modern_style_sheet', async () => {
    stubApi({ 'GET launch': launch('og') });
    const { default: App } = await import('./App.svelte');

    render(App);

    await waitFor(() => expect(document.querySelector('.org-header')).not.toBeNull(), {
      timeout: 10_000,
    });
    expect(document.documentElement.dataset.layout).toBe('og');
    expect(loadModernStyles).not.toHaveBeenCalled();
  }, 30_000);

  it('a_modern_screen_outside_the_shell_waits_for_the_modern_style_sheet', async () => {
    // The set-up wizard is drawn without the modern shell: it waits for the modern rules.
    stubApi({
      'GET launch': launch('modern', 'wizard'),
      'GET status': { data: { database: { type: 'sqlite' } } },
    });
    const { default: App } = await import('./App.svelte');

    render(App);

    await waitFor(() => expect(loadModernStyles).toHaveBeenCalled(), { timeout: 10_000 });
    expect(document.documentElement.dataset.layout).toBe('modern');
    expect(document.querySelector('.loading-spinner')).not.toBeNull();
    expect(document.querySelector('.org-wizard')).toBeNull();

    styles.loaded();

    await waitFor(() => expect(document.querySelector('.org-wizard')).not.toBeNull());
    expect(document.querySelector('.loading-spinner')).toBeNull();
  }, 30_000);
});
