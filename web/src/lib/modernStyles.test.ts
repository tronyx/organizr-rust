import { afterEach, describe, expect, it, vi } from 'vitest';

// Vitest gives a style sheet an empty address: the test gives it the address of a build.
vi.mock('../modern.css?url', () => ({ default: './assets/modern-abc.css' }));

/** The link of the modern style sheet, if the page has one. */
const modernLink = () => document.querySelector<HTMLLinkElement>('link#org-modern-css');

/** A fresh copy of the module, which has not linked the style sheet yet. */
async function freshModule() {
  vi.resetModules();
  return import('./modernStyles');
}

afterEach(() => {
  document.head.innerHTML = '';
});

describe('loadModernStyles', () => {
  it('links_the_modern_style_sheet_and_waits_for_it', async () => {
    const { loadModernStyles } = await freshModule();
    expect(modernLink()).toBeNull();

    let done = false;
    const loading = loadModernStyles().then(() => {
      done = true;
    });
    const link = modernLink();
    expect(link?.rel).toBe('stylesheet');
    expect(link?.getAttribute('href')).toBe('./assets/modern-abc.css');
    await Promise.resolve();
    expect(done).toBe(false);

    link?.dispatchEvent(new Event('load'));
    await loading;
    expect(done).toBe(true);
  });

  it('links_the_sheet_once', async () => {
    const { loadModernStyles } = await freshModule();
    const first = loadModernStyles();
    const second = loadModernStyles();
    expect(second).toBe(first);
    expect(document.querySelectorAll('link#org-modern-css')).toHaveLength(1);
  });

  it('links_the_sheet_before_the_custom_css_of_the_admin', async () => {
    const custom = document.createElement('link');
    custom.id = 'org-custom-css';
    custom.rel = 'stylesheet';
    document.head.append(custom);
    const { loadModernStyles } = await freshModule();

    void loadModernStyles();

    expect(modernLink()?.nextElementSibling).toBe(custom);
  });

  it('a_sheet_that_does_not_load_fails_and_a_later_call_gives_the_same_failure', async () => {
    // The page then keeps the original shell (`App.svelte`), so nothing asks again.
    const { loadModernStyles } = await freshModule();
    const failed = loadModernStyles();
    modernLink()?.dispatchEvent(new Event('error'));
    await expect(failed).rejects.toThrow(/modern/);
    expect(modernLink()).toBeNull();

    expect(loadModernStyles()).toBe(failed);
    expect(modernLink()).toBeNull();
  });
});
