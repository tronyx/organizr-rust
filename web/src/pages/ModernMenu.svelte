<script lang="ts">
  // The menu at the left of the modern layout: the tabs that the caller sees, under their
  // categories. A category heading opens and closes its group.
  import Picture from '../components/Picture.svelte';
  import { buildMenu, opensNewWindow } from '../lib/menu';
  import { tabHash } from '../lib/route.svelte';
  import type { MenuCategory, MenuTab, UnsortedPlace } from '../lib/types';

  let {
    tabs,
    categories,
    unsortedPlace,
    shown,
    onpick,
  }: {
    tabs: MenuTab[];
    categories: MenuCategory[];
    /** Whether the tabs without a heading stand above the categories or below them. */
    unsortedPlace: UnsortedPlace;
    /** The id of the tab that the main area shows. */
    shown: number | undefined;
    /** Called when the user picks a tab. */
    onpick?: () => void;
  } = $props();

  const menu = $derived(buildMenu(tabs, categories, unsortedPlace));
</script>

{#snippet entry(tab: MenuTab)}
  <li>
    {#if opensNewWindow(tab) && tab.access_url}
      <a
        href={tab.access_url}
        target="_blank"
        rel="noopener noreferrer"
        aria-label={tab.name}
        translate="no"
        onclick={onpick}
      >
        <Picture image={tab.image} name={tab.name} />
        <span class="truncate">{tab.name}</span>
        <i class="oi oi-themify ti-new-window ml-auto text-xs" aria-hidden="true"></i>
      </a>
    {:else}
      <a
        href={tabHash(tab.name)}
        aria-label={tab.name}
        translate="no"
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

<nav aria-label="Tabs" class="org-menu">
  <ul>
    {#if menu.unsortedPlace === 'top'}
      {@render unsorted()}
    {/if}
    {#each menu.groups as group (group.category.id)}
      <li>
        <!-- The group of the shown tab is open; the others start closed, as in the OG menu. -->
        <details class="m-group" open={group.tabs.some((tab) => tab.id === shown)}>
          <summary aria-label={group.category.category}>
            <Picture image={group.category.image} name={group.category.category} />
            <span class="truncate">{group.category.category}</span>
            <span class="org-label">{group.tabs.length}</span>
            <i class="oi oi-themify ti-angle-right org-arrow" aria-hidden="true"></i>
          </summary>
          <!-- A wrapper, so the rules of the OG strip for `details > ul` never match here. -->
          <div>
            <ul>
              {#each group.tabs as tab (tab.id)}
                {@render entry(tab)}
              {/each}
            </ul>
          </div>
        </details>
      </li>
    {/each}
    {#if menu.unsortedPlace === 'bottom'}
      {@render unsorted()}
    {/if}
  </ul>
</nav>
