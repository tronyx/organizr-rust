<script lang="ts">
  // The frames that show the applications of the tabs. A frame stays loaded when the
  // user goes to another tab, so the application in it keeps its state.
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

  // The ids of the tabs that the user has opened.
  let opened = $state<number[]>([]);

  $effect(() => {
    if (shown !== undefined && !opened.includes(shown)) {
      opened.push(shown);
    }
  });

  // A tab gets a frame once it was opened, or at once when it is marked "preload". A
  // tab that the menu has no more loses its frame.
  const frames = $derived(
    tabs.filter(
      (tab) =>
        tab.type === 1 &&
        !opensNewWindow(tab) &&
        Boolean(tab.access_url) &&
        (opened.includes(tab.id) || tab.preload === 1),
    ),
  );
</script>

{#each frames as tab (tab.id)}
  <iframe
    class="absolute inset-0 size-full border-0"
    class:hidden={tab.id !== shown}
    src={tab.access_url}
    title={tab.name}
    allow="fullscreen"
  ></iframe>
{/each}
