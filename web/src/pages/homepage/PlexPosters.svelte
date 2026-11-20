<script lang="ts">
  // A row of posters of the items of a media server. A poster opens the details of
  // its item.
  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import PosterRow from './PosterRow.svelte';
  import {
    linkOf,
    type MediaService,
    pictureAddress,
    type PlexAnswer,
    type PlexItem,
    SERVICE_NAMES,
    subtitleOf,
  } from '../../lib/plex';

  let {
    items,
    label,
    title = label,
    service = 'plex',
    detailsPath = `homepage/${service}/metadata`,
    serverName = SERVICE_NAMES[service],
  }: {
    items: PlexItem[];
    label: string;
    /** The title above the row. */
    title?: string;
    /** The server of the items. */
    service?: MediaService;
    /** The route that gives the details of an item, when it is not the one of the server. */
    detailsPath?: string;
    /** The name of the server on the button that opens an item there. */
    serverName?: string;
  } = $props();

  // The item is kept as it is, so it can be told apart from the item of a later click.
  let open = $state.raw<PlexItem | null>(null);
  let details = $state<PlexItem | null>(null);
  let error = $state<string | null>(null);

  /** Opens the dialog with what the list knows, and loads what the server knows more. */
  async function show(item: PlexItem) {
    open = item;
    details = null;
    error = null;
    if (!item.metadataKey) {
      return;
    }
    try {
      const answer = await api<PlexAnswer | null>('POST', detailsPath, {
        key: item.metadataKey,
      });
      if (open === item) {
        details = answer.data?.content?.[0] ?? null;
      }
    } catch (failure) {
      error = failureText(failure);
    }
  }

  const shown = $derived(details ?? open);
  // The list names the address of an item. Some lists do not, and the details do.
  const link = $derived(open ? (linkOf(open) ?? (details ? linkOf(details) : null)) : null);
  const genres = $derived(Array.isArray(shown?.metadata?.genres) ? shown.metadata.genres : []);
  const actors = $derived(Array.isArray(shown?.metadata?.actors) ? shown.metadata.actors : []);
</script>

<PosterRow {title} {label}>
  {#each items as item, index (index)}
    {@const picture = pictureAddress(item.imageURL)}
    <li class="org-poster">
      <button type="button" title="{item.title} {subtitleOf(item)}" onclick={() => show(item)}>
        {#if picture}
          <img src={picture} alt="" loading="lazy" />
        {:else}
          <span class="org-poster-empty" aria-hidden="true">{[...item.title][0] ?? '?'}</span>
        {/if}
        <span class="sr-only">{item.title}</span>
        <span class="sr-only">{subtitleOf(item)}</span>
      </button>
    </li>
  {/each}
</PosterRow>

{#if open && shown}
  {@const picture = pictureAddress(shown.nowPlayingImageURL)}
  <Dialog title={open.title} onclose={() => (open = null)}>
    {#if subtitleOf(open)}
      <p class="opacity-70">{subtitleOf(open)}</p>
    {/if}
    {#if picture}
      <img class="my-2 max-h-64 w-full rounded object-cover" src={picture} alt="" />
    {/if}
    {#if error}
      <div role="alert" class="alert alert-error my-2"><span>{error}</span></div>
    {/if}
    <p class="my-2 flex flex-wrap gap-1">
      {#if shown.metadata?.year}
        <span class="badge badge-outline">{shown.metadata.year}</span>
      {/if}
      {#if shown.metadata?.rating}
        <span class="badge badge-outline">Rating {shown.metadata.rating}</span>
      {/if}
      {#if shown.metadata?.studio}
        <span class="badge badge-outline">{shown.metadata.studio}</span>
      {/if}
      {#each genres as genre (genre)}
        <span class="badge badge-ghost">{genre}</span>
      {/each}
    </p>
    {#if shown.metadata?.tagline}
      <p class="text-sm italic">{shown.metadata.tagline}</p>
    {/if}
    {#if shown.metadata?.summary || shown.summary}
      <p class="text-sm">{shown.metadata?.summary || shown.summary}</p>
    {/if}
    {#if actors.length > 0}
      <p class="mt-2 text-sm opacity-70">
        With {actors
          .slice(0, 6)
          .map((actor) => actor.name)
          .join(', ')}
      </p>
    {/if}
    <div class="modal-action">
      {#if link}
        <a class="btn" href={link} target="_blank" rel="noopener noreferrer">
          Open in {serverName}
        </a>
      {/if}
      <button type="button" class="btn btn-primary" onclick={() => (open = null)}>Close</button>
    </div>
  </Dialog>
{/if}
