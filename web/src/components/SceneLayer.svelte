<script lang="ts">
  // The animated scene of a scene theme, fixed behind the page of the modern shell. It loads
  // the scene of the theme only when that theme is on, and goes away with it. The art is
  // decoration: hidden from assistive technology, and it takes no clicks (`.m-scene` in
  // `app.css`). While the tab is hidden, or while the page covers the scene (any page but
  // the clear ones of `SCENE_PAGES`), `m-scene-paused` stops every animation; a person who asked the
  // system for less motion gets the scene still (each scene's own style).
  import type { Component } from 'svelte';

  import { MODERN_THEMES, sceneOf } from '../lib/look';
  import { session } from '../lib/session.svelte';
  import { SCENE_LOADERS, type SceneLoaders } from '../scenes';

  let {
    loaders = SCENE_LOADERS,
    covered = false,
  }: {
    loaders?: SceneLoaders;
    /** The page hides all of the scene (any page but those of `SCENE_PAGES`): stand still. */
    covered?: boolean;
  } = $props();

  const scene = $derived(sceneOf(session.look));
  // The theme, for the class `m-scene-<theme>` that custom CSS can use.
  const theme = $derived(
    scene ? (MODERN_THEMES.find((entry) => entry.scene === scene)?.id ?? scene) : null,
  );

  // The component of the scene that has loaded, with the id it belongs to.
  let loaded = $state<{ id: string; component: Component } | null>(null);

  $effect(() => {
    const id = scene;
    if (!id || loaded?.id === id) {
      return;
    }
    const load = loaders[id];
    if (!load) {
      return;
    }
    let wanted = true;
    load().then(
      (module) => {
        if (wanted) {
          loaded = { id, component: module.default };
        }
      },
      // A scene that does not load (an old cached page, a lost connection) leaves the
      // page as it is: opaque, with no scene behind it.
      () => {},
    );
    return () => {
      wanted = false;
    };
  });

  let hidden = $state(typeof document !== 'undefined' && document.hidden);

  $effect(() => {
    const update = () => {
      hidden = document.hidden;
    };
    document.addEventListener('visibilitychange', update);
    return () => document.removeEventListener('visibilitychange', update);
  });

  const Scene = $derived(loaded && loaded.id === scene ? loaded.component : null);
</script>

{#if Scene && theme}
  <div
    class={['m-scene', `m-scene-${theme}`, (hidden || covered) && 'm-scene-paused']}
    aria-hidden="true"
  >
    <Scene />
  </div>
{/if}
