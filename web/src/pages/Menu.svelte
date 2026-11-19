<script lang="ts">
  // The menu at the left: the tabs that the caller sees, under their categories.
  import Picture from '../components/Picture.svelte';
  import { buildMenu, opensNewWindow } from '../lib/menu';
  import { tabHash } from '../lib/route.svelte';
  import type { MenuCategory, MenuTab } from '../lib/types';

  let {
    tabs,
    categories,
    shown,
    onpick,
  }: {
    tabs: MenuTab[];
    categories: MenuCategory[];
    /** The id of the tab that the main area shows. */
    shown: number | undefined;
    /** Called when the user picks a tab. */
    onpick?: () => void;
  } = $props();

  const menu = $derived(buildMenu(tabs, categories));
</script>

{#snippet entry(tab: MenuTab)}
  <li>
    {#if opensNewWindow(tab) && tab.access_url}
      <a href={tab.access_url} target="_blank" rel="noopener noreferrer" onclick={onpick}>
        <Picture image={tab.image} name={tab.name} />
        <span class="truncate">{tab.name}</span>
        <span class="badge badge-ghost badge-sm" title="Opens in a new window">new window</span>
      </a>
    {:else}
      <a
        href={tabHash(tab.name)}
        class:menu-active={tab.id === shown}
        aria-current={tab.id === shown ? 'page' : undefined}
        onclick={onpick}
      >
        <Picture image={tab.image} name={tab.name} />
        <span class="truncate">{tab.name}</span>
      </a>
    {/if}
  </li>
{/snippet}

{#snippet unsorted()}
  {#each menu.unsorted as tab (tab.id)}
    {@render entry(tab)}
  {/each}
{/snippet}

<nav aria-label="Tabs">
  <ul class="menu w-full">
    {#if menu.unsortedPlace === 'top'}
      {@render unsorted()}
    {/if}
    {#each menu.groups as group (group.category.id)}
      <li>
        <details open>
          <summary>
            <Picture image={group.category.image} name={group.category.category} />
            <span class="truncate">{group.category.category}</span>
          </summary>
          <ul>
            {#each group.tabs as tab (tab.id)}
              {@render entry(tab)}
            {/each}
          </ul>
        </details>
      </li>
    {/each}
    {#if menu.unsortedPlace === 'bottom'}
      {@render unsorted()}
    {/if}
  </ul>
</nav>
