<script lang="ts">
  // A row of posters of Plex items. A poster opens the details of its item.
  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import { pictureAddress, type PlexAnswer, type PlexItem, subtitleOf } from '../../lib/plex';

  let { items, label }: { items: PlexItem[]; label: string } = $props();

  let open = $state<PlexItem | null>(null);
  let details = $state<PlexItem | null>(null);
  let error = $state<string | null>(null);

  /** Opens the dialog with what the list knows, and loads what Plex knows more. */
  async function show(item: PlexItem) {
    open = item;
    details = null;
    error = null;
    if (!item.metadataKey) {
      return;
    }
    try {
      const answer = await api<PlexAnswer | null>('POST', 'homepage/plex/metadata', {
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
  const genres = $derived(Array.isArray(shown?.metadata?.genres) ? shown.metadata.genres : []);
  const actors = $derived(Array.isArray(shown?.metadata?.actors) ? shown.metadata.actors : []);
</script>

<ul class="flex gap-3 overflow-x-auto pb-2" aria-label={label}>
  {#each items as item, index (index)}
    {@const picture = pictureAddress(item.imageURL)}
    <li class="w-28 shrink-0">
      <button type="button" class="w-full text-left" onclick={() => show(item)}>
        {#if picture}
          <img
            class="bg-base-300 aspect-[2/3] w-full rounded object-cover"
            src={picture}
            alt=""
            loading="lazy"
          />
        {:else}
          <span class="bg-base-300 grid aspect-[2/3] w-full place-items-center rounded text-2xl">
            {[...item.title][0] ?? '?'}
          </span>
        {/if}
        <span class="mt-1 block truncate text-sm font-medium">{item.title}</span>
        <span class="block truncate text-xs opacity-70">{subtitleOf(item)}</span>
      </button>
    </li>
  {/each}
</ul>

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
      {#if /^https?:\/\//.test(open.address)}
        <a class="btn" href={open.address} target="_blank" rel="noopener noreferrer">
          Open in Plex
        </a>
      {/if}
      <button type="button" class="btn btn-primary" onclick={() => (open = null)}>Close</button>
    </div>
  </Dialog>
{/if}
