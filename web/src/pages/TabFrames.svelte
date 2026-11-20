<script lang="ts">
  // The frames that show the applications of the tabs. A frame stays loaded when the
  // user goes to another tab, so the application in it keeps its state.
  import { frames } from '../lib/frames.svelte';
  import { opensNewWindow } from '../lib/menu';
  import type { MenuTab } from '../lib/types';

  let {
    tabs,
    shown,
  }: {
    tabs: MenuTab[];
    /** The id of the tab whose frame is on top. No frame is on top without it. */
    shown: number | undefined;
  } = $props();

  $effect(() => {
    if (shown !== undefined) {
      frames.open(shown);
    }
  });

  // A tab gets a frame once it was opened, or at once when it is marked "preload". A
  // tab that the menu has no more loses its frame.
  const loaded = $derived(
    tabs.filter(
      (tab) =>
        tab.type === 1 &&
        !opensNewWindow(tab) &&
        Boolean(tab.access_url) &&
        (frames.opened.includes(tab.id) || tab.preload === 1),
    ),
  );
</script>

<!-- The key holds the number of loads, so "load again" makes a new frame. -->
{#each loaded as tab (`${tab.id}:${frames.loads[tab.id] ?? 0}`)}
  <iframe
    class="absolute inset-0 size-full border-0"
    class:hidden={tab.id !== shown}
    src={tab.access_url}
    title={tab.name}
    allow="fullscreen"
  ></iframe>
{/each}
