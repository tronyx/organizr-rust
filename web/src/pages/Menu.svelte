<script lang="ts">
  // The menu at the left: the tabs that the caller sees, under their categories.

  import Picture from '../components/Picture.svelte';
  import { buildMenu, opensNewWindow, tabAddress } from '../lib/menu';
  import { pingStatus, pingTime, type PingResult } from '../lib/pings';
  import { tabHash } from '../lib/route.svelte';
  import type { MenuCategory, MenuTab, UnsortedPlace } from '../lib/types';

  let {
    tabs,
    categories,
    unsortedPlace,
    pings = {},
    times = false,
    shown,
    onpick,
    variant = 'og',
    collapsed = false,
  }: {
    tabs: MenuTab[];
    categories: MenuCategory[];
    /** Whether the tabs without a heading stand above the categories or below them. */
    unsortedPlace: UnsortedPlace;
    /** What the server answered for the addresses to check of the tabs. */
    pings?: Record<string, PingResult>;
    /** Whether to show the time of each answer, which the setting Show Ping Time turns on. */
    times?: boolean;
    /** The id of the tab that the main area shows. */
    shown: number | undefined;
    /** Called when the user picks a tab. */
    onpick?: () => void;
    /**
     * `og` is the strip of pictures that opens under the pointer. `modern` is the column of
     * the modern layout: it never hides the words, and its pictures are the small size.
     */
    variant?: 'og' | 'modern';
    /**
     * Only for `modern`: the menu is a column of icons, as the closed sidebar of PHP
     * Organizr. A tab shows its picture; a category shows its picture, and the pictures
     * of its tabs when it is open. The names show when the menu opens under the pointer.
     */
    collapsed?: boolean;
  } = $props();

  const og = $derived(variant === 'og');
  const icons = $derived(!og && collapsed);

  const menu = $derived(buildMenu(tabs, categories, unsortedPlace));
</script>

<!-- `push` moves the dot to the end of the row; a modern row with the new-window icon has that icon do it. -->
{#snippet status(tab: MenuTab, push: boolean)}
  {@const answer = pingStatus(tab, pings)}
  {@const time = times ? pingTime(tab, pings) : null}
  {#if time !== null && !icons}
    <small class={['org-label', og && 'org-wide']}>{time}</small>
  {/if}
  {#if answer === 'up'}
    <span
      class={[
        'status status-success',
        og && 'org-wide',
        push && !icons && 'ml-auto',
        icons && 'm-dot',
      ]}
      role="img"
      aria-label="answers"
    ></span>
  {:else if answer === 'down'}
    <span
      class={[
        'status status-error',
        og && 'org-wide',
        push && !icons && 'ml-auto',
        icons && 'm-dot',
      ]}
      role="img"
      aria-label="does not answer"
    ></span>
  {/if}
{/snippet}

{#snippet entry(tab: MenuTab)}
  <li>
    {#if opensNewWindow(tab) && tabAddress(tab)}
      <a
        onclick={onpick}
        href={tabAddress(tab)}
        target="_blank"
        rel="noopener noreferrer"
        aria-label={tab.name}
        translate="no"
      >
        <Picture image={tab.image} name={tab.name} large={og} />
        {#if !icons}
          <span class={[og && 'org-wide', 'truncate']}>{tab.name}</span>
          <i
            class={['oi oi-themify ti-new-window', og && 'org-wide', 'text-xs', !og && 'ml-auto']}
            aria-hidden="true"
            title="Opens in a new window"
          ></i>
        {/if}
        {@render status(tab, og)}
      </a>
    {:else}
      <a
        onclick={onpick}
        href={tabHash(tab.name)}
        aria-label={tab.name}
        translate="no"
        aria-current={tab.id === shown ? 'page' : undefined}
      >
        <Picture image={tab.image} name={tab.name} large={og} />
        {#if !icons}
          <span class={[og && 'org-wide', 'truncate']}>{tab.name}</span>
        {/if}
        {@render status(tab, true)}
      </a>
    {/if}
  </li>
{/snippet}

{#snippet list(items: MenuTab[])}
  <ul>
    {#each items as tab (tab.id)}
      {@render entry(tab)}
    {/each}
  </ul>
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
      <!-- The column of icons keeps the same parts, so a category stays open or closed
           when the menu opens and closes. -->
      <li>
        <!-- A category is closed at first, as in PHP Organizr. The one of the shown tab is open. -->
        <details
          class={og ? undefined : 'm-group'}
          open={group.tabs.some((tab) => tab.id === shown)}
        >
          <summary aria-label={group.category.category}>
            <Picture image={group.category.image} name={group.category.category} large={og} />
            {#if !icons}
              <span class={[og && 'org-wide', 'truncate']}>{group.category.category}</span>
              <span class={['org-label', og && 'org-wide']}>{group.tabs.length}</span>
              <i
                class={['oi oi-themify ti-angle-right org-arrow', og && 'org-wide']}
                aria-hidden="true"
              ></i>
            {/if}
          </summary>
          {#if og}
            {@render list(group.tabs)}
          {:else}
            <!-- A wrapper, so the rules of the OG strip for `details > ul` never match here. -->
            <div>
              {@render list(group.tabs)}
            </div>
          {/if}
        </details>
      </li>
    {/each}
    {#if menu.unsortedPlace === 'bottom'}
      {@render unsorted()}
    {/if}
  </ul>
</nav>
