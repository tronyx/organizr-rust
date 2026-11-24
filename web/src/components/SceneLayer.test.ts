import { render, waitFor } from '@testing-library/svelte';
import { flushSync, type Component } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { Look } from '../lib/look';
import { session } from '../lib/session.svelte';
import type { Launch } from '../lib/types';
import type { SceneLoaders } from '../scenes';
import SceneLayer from './SceneLayer.svelte';

/** A stand-in for a scene: one element that the tests can find. */
const FakeScene = ((anchor: Node) => {
  const art = document.createElement('div');
  art.className = 'fake-scene-art';
  anchor.parentNode?.insertBefore(art, anchor);
}) as unknown as Component;

function useLook(look: Partial<Look>) {
  session.launch = {
    look: { layout: 'modern', theme: 'organizr', colourSet: 'dark', source: 'user', ...look },
  } as unknown as Launch;
  flushSync();
}

function loaders(): SceneLoaders & { nocturne: ReturnType<typeof vi.fn> } {
  return { nocturne: vi.fn(async () => ({ default: FakeScene })) };
}

function setHidden(hidden: boolean) {
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => hidden });
  document.dispatchEvent(new Event('visibilitychange'));
  flushSync();
}

afterEach(() => {
  session.launch = null;
  setHidden(false);
});

describe('SceneLayer', () => {
  it('renders_nothing_for_organizr_and_plex', async () => {
    const map = loaders();
    useLook({ theme: 'organizr', colourSet: 'dark' });
    const { container } = render(SceneLayer, { loaders: map });
    useLook({ theme: 'plex', colourSet: 'light' });
    await Promise.resolve();

    expect(container.querySelector('.m-scene')).toBeNull();
    expect(map.nocturne).not.toHaveBeenCalled();
  });

  it('renders_nothing_on_og', () => {
    const map = loaders();
    useLook({ layout: 'og', theme: 'nocturne', colourSet: 'amethyst' });
    const { container } = render(SceneLayer, { loaders: map });

    expect(container.querySelector('.m-scene')).toBeNull();
    expect(map.nocturne).not.toHaveBeenCalled();
  });

  it('loads_the_nocturne_scene_hidden_from_assistive_technology', async () => {
    const map = loaders();
    useLook({ theme: 'nocturne', colourSet: 'crimson' });
    const { container } = render(SceneLayer, { loaders: map });

    const scene = await waitFor(() => {
      const found = container.querySelector('.m-scene');
      expect(found).not.toBeNull();
      return found as HTMLElement;
    });
    expect(map.nocturne).toHaveBeenCalledTimes(1);
    expect(scene).toHaveAttribute('aria-hidden', 'true');
    expect(scene).toHaveClass('m-scene', 'm-scene-nocturne');
    expect(scene).not.toHaveClass('m-scene-paused');
    expect(scene.querySelector('.fake-scene-art')).not.toBeNull();
    // A change of colour set keeps the loaded scene.
    useLook({ theme: 'nocturne', colourSet: 'moonlit' });
    expect(container.querySelector('.m-scene-nocturne')).not.toBeNull();
    expect(map.nocturne).toHaveBeenCalledTimes(1);
  });

  it('a_hidden_tab_pauses_the_scene', async () => {
    useLook({ theme: 'nocturne', colourSet: 'amethyst' });
    const { container } = render(SceneLayer, { loaders: loaders() });
    await waitFor(() => expect(container.querySelector('.m-scene')).not.toBeNull());

    setHidden(true);
    expect(container.querySelector('.m-scene')).toHaveClass('m-scene-paused');
    setHidden(false);
    expect(container.querySelector('.m-scene')).not.toHaveClass('m-scene-paused');
  });

  it('switching_to_plex_removes_the_scene', async () => {
    useLook({ theme: 'nocturne', colourSet: 'amethyst' });
    const { container } = render(SceneLayer, { loaders: loaders() });
    await waitFor(() => expect(container.querySelector('.m-scene')).not.toBeNull());

    useLook({ theme: 'plex', colourSet: 'dark' });

    expect(container.querySelector('.m-scene')).toBeNull();
    expect(container.querySelector('.fake-scene-art')).toBeNull();
  });

  it('a_scene_that_arrives_after_a_switch_away_stays_out', async () => {
    let finish: (value: { default: Component }) => void = () => {};
    const slow: SceneLoaders = {
      nocturne: () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    };
    useLook({ theme: 'nocturne', colourSet: 'amethyst' });
    const { container } = render(SceneLayer, { loaders: slow });
    useLook({ theme: 'emby', colourSet: 'dark' });
    finish({ default: FakeScene });
    await Promise.resolve();
    flushSync();

    expect(container.querySelector('.m-scene')).toBeNull();
  });

  it('a_scene_that_fails_to_load_leaves_the_page_as_it_is', async () => {
    const failing: SceneLoaders = { nocturne: () => Promise.reject(new Error('gone')) };
    useLook({ theme: 'nocturne', colourSet: 'amethyst' });
    const { container } = render(SceneLayer, { loaders: failing });
    await Promise.resolve();
    flushSync();

    expect(container.querySelector('.m-scene')).toBeNull();
  });

  it('the_modern_shell_mounts_the_scene_layer_first', async () => {
    const { readFileSync } = await import('node:fs');
    const { resolve } = await import('node:path');
    const shell = readFileSync(resolve(__dirname, '../pages/ModernShell.svelte'), 'utf8');
    const markup = shell.slice(shell.indexOf('</script>') + '</script>'.length);
    const firstTag = /<[A-Za-z][\w.]*/.exec(markup);
    expect(firstTag?.[0]).toBe('<SceneLayer');
    expect(shell).toContain("import SceneLayer from '../components/SceneLayer.svelte';");
  });
});
