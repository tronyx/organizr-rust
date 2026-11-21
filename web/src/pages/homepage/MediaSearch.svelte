<script lang="ts">
  // The media search of the top bar of PHP Organizr: a pop-up with "Search My Media"
  // that asks the media server as the person types, and shows the matches as posters.
  // A poster opens the details of its item.
  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import {
    type MediaService,
    pictureAddress,
    type PlexAnswer,
    type PlexItem,
    SERVICE_NAMES,
  } from '../../lib/plex';
  import MediaDetails from './MediaDetails.svelte';

  let { service }: { service: MediaService } = $props();

  /** How long the search waits after the last key, as PHP Organizr does. */
  const PAUSE = 500;

  let open = $state(false);
  let query = $state('');
  let results = $state<PlexItem[] | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let chosen = $state.raw<PlexItem | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function typed() {
    clearTimeout(timer);
    timer = setTimeout(() => void search(), PAUSE);
  }

  async function search() {
    const asked = query.trim();
    if (asked === '') {
      results = null;
      return;
    }
    busy = true;
    try {
      const answer = await api<PlexAnswer>(
        'GET',
        `homepage/${service}/search/${encodeURIComponent(asked)}`,
      );
      if (asked === query.trim()) {
        results = Array.isArray(answer.data.content) ? answer.data.content : [];
        error = null;
      }
    } catch (failure) {
      error = failureText(failure);
      results = null;
    } finally {
      busy = false;
    }
  }
</script>

<button
  type="button"
  class="org-header-link"
  aria-label="Search My Media"
  onclick={() => (open = true)}
>
  <i class="oi oi-themify ti-search" aria-hidden="true"></i>
</button>

{#if open}
  <Dialog title="Search My Media" onclose={() => (open = false)}>
    <input
      class="org-form-control"
      type="search"
      placeholder="Search My Media"
      aria-label="Search My Media"
      bind:value={query}
      oninput={typed}
    />
    {#if busy}
      <span class="loading loading-spinner loading-sm mt-3" aria-label="Searching"></span>
    {/if}
    {#if error}
      <div role="alert" class="alert alert-error mt-3"><span>{error}</span></div>
    {/if}
    {#if results}
      {#if results.length === 0}
        <p class="mt-3 text-sm">No results for {query.trim()}</p>
      {:else}
        <ul class="org-search-results" aria-label="Results in {SERVICE_NAMES[service]}">
          {#each results as result, index (index)}
            {@const picture = pictureAddress(result.imageURL)}
            <li>
              <button type="button" onclick={() => (chosen = result)}>
                {#if picture}
                  <img src={picture} alt="" loading="lazy" />
                {:else}
                  <span class="org-poster-empty" aria-hidden="true">
                    {[...result.title][0] ?? '?'}
                  </span>
                {/if}
                <span class="truncate text-xs">{result.title}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </Dialog>
{/if}

{#if chosen}
  <MediaDetails
    item={chosen}
    detailsPath="homepage/{service}/metadata"
    serverName={SERVICE_NAMES[service]}
    onclose={() => (chosen = null)}
  />
{/if}
