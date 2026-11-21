<script lang="ts">
  // The details of an item of a media server in a dialog: what the list knows at once,
  // then what the server knows more, as the metadata pop-up of PHP Organizr.
  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import {
    linkOf,
    pictureAddress,
    type PlexAnswer,
    type PlexItem,
    subtitleOf,
  } from '../../lib/plex';

  let {
    item,
    detailsPath,
    serverName,
    onclose,
  }: {
    item: PlexItem;
    /** The route that gives the details of an item. */
    detailsPath: string;
    /** The name of the server on the button that opens the item there. */
    serverName: string;
    onclose: () => void;
  } = $props();

  let details = $state<PlexItem | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const asked = item;
    details = null;
    error = null;
    if (!asked.metadataKey) {
      return;
    }
    api<PlexAnswer | null>('POST', detailsPath, { key: asked.metadataKey })
      .then((answer) => {
        if (asked === item) {
          details = answer.data?.content?.[0] ?? null;
        }
      })
      .catch((failure: unknown) => {
        error = failureText(failure);
      });
  });

  const shown = $derived(details ?? item);
  // The list names the address of an item. Some lists do not, and the details do.
  const link = $derived(linkOf(item) ?? (details ? linkOf(details) : null));
  const genres = $derived(Array.isArray(shown.metadata?.genres) ? shown.metadata.genres : []);
  const actors = $derived(Array.isArray(shown.metadata?.actors) ? shown.metadata.actors : []);
  const picture = $derived(pictureAddress(shown.nowPlayingImageURL));
</script>

<Dialog title={item.title} {onclose}>
  {#if subtitleOf(item)}
    <p class="opacity-70">{subtitleOf(item)}</p>
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
    <button type="button" class="btn btn-primary" onclick={onclose}>Close</button>
  </div>
</Dialog>
