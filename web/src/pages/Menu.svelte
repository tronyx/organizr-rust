<script lang="ts">
  // The menu at the left: the tabs that the caller sees, under their categories.
  import Picture from '../components/Picture.svelte';
  import { buildMenu, opensNewWindow } from '../lib/menu';
  import { pingStatus, pingTime, type PingResult } from '../lib/pings';
  import { tabHash } from '../lib/route.svelte';
  import type { MenuCategory, MenuTab, UnsortedPlace } from '../lib/types';

  let {
    tabs,
    categories,
    unsortedPlace,
    pings = {},
    shown,
    onpick,
  }: {
    tabs: MenuTab[];
    categories: MenuCategory[];
    /** Whether the tabs without a heading stand above the categories or below them. */
    unsortedPlace: UnsortedPlace;
    /** What the server answered for the addresses to check of the tabs. */
    pings?: Record<string, PingResult>;
    /** The id of the tab that the main area shows. */
    shown: number | undefined;
    /** Called when the user picks a tab. */
    onpick?: () => void;
  } = $props();

  const menu = $derived(buildMenu(tabs, categories, unsortedPlace));
</script>

{#snippet status(tab: MenuTab)}
  {@const answer = pingStatus(tab, pings)}
  {@const time = pingTime(tab, pings)}
  {#if time !== null}
    <small class="org-label org-wide">{time}</small>
  {/if}
  {#if answer === 'up'}
    <span class="status status-success org-wide ml-auto" role="img" aria-label="answers"></span>
  {:else if answer === 'down'}
    <span class="status status-error org-wide ml-auto" role="img" aria-label="does not answer"
    ></span>
  {/if}
{/snippet}

{#snippet entry(tab: MenuTab)}
  <li>
    {#if opensNewWindow(tab) && tab.access_url}
      <a
        href={tab.access_url}
        target="_blank"
        rel="noopener noreferrer"
        aria-label={tab.name}
        onclick={onpick}
      >
        <Picture image={tab.image} name={tab.name} large />
        <span class="org-wide truncate">{tab.name}</span>
        <i
          class="oi oi-themify ti-new-window org-wide text-xs"
          aria-hidden="true"
          title="Opens in a new window"
        ></i>
        {@render status(tab)}
      </a>
    {:else}
      <a
        href={tabHash(tab.name)}
        aria-label={tab.name}
        aria-current={tab.id === shown ? 'page' : undefined}
        onclick={onpick}
      >
        <Picture image={tab.image} name={tab.name} large />
        <span class="org-wide truncate">{tab.name}</span>
        {@render status(tab)}
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
        <!-- A category is closed at first, as in PHP Organizr. The one of the shown tab is open. -->
        <details open={group.tabs.some((tab) => tab.id === shown)}>
          <summary aria-label={group.category.category}>
            <Picture image={group.category.image} name={group.category.category} large />
            <span class="org-wide truncate">{group.category.category}</span>
            <span class="org-label org-wide">{group.tabs.length}</span>
            <i class="oi oi-themify ti-angle-right org-arrow org-wide" aria-hidden="true"></i>
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
