<script lang="ts">
  // Plex on the homepage: what plays now, what was added last, or the playlists.
  import { itemData } from '../../lib/itemData.svelte';
  import { pictureAddress, type PlexAnswer, type PlexItem, streamText } from '../../lib/plex';
  import PlexPosters from './PlexPosters.svelte';

  /** A playlist: its title, and its videos under the keys "0", "1" and so on. */
  type Playlist = { title: string } & Record<string, PlexItem | string>;

  interface PlaylistAnswer {
    content?: Record<string, Playlist>;
  }

  let { part }: { part: 'streams' | 'recent' | 'playlists' } = $props();

  // svelte-ignore state_referenced_locally
  const refresh = part === 'streams' ? 'homepageStreamRefresh' : 'homepageRecentRefresh';
  // svelte-ignore state_referenced_locally
  const item = itemData<PlexAnswer | PlaylistAnswer>(`homepage/plex/${part}`, refresh);

  const items = $derived(
    part !== 'playlists' && Array.isArray(item.data?.content) ? item.data.content : [],
  );
  const playlists = $derived(
    part === 'playlists' && item.data?.content && !Array.isArray(item.data.content)
      ? Object.entries(item.data.content)
      : [],
  );

  let chosen = $state<string | null>(null);
  const playlist = $derived(
    playlists.find(([name]) => name === chosen)?.[1] ?? playlists[0]?.[1] ?? null,
  );

  /** The videos of a playlist, in their order. */
  function videosOf(list: Playlist): PlexItem[] {
    return Object.entries(list)
      .filter(
        (entry): entry is [string, PlexItem] =>
          /^\d+$/.test(entry[0]) && typeof entry[1] === 'object',
      )
      .sort(([a], [b]) => Number(a) - Number(b))
      .map(([, video]) => video);
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Plex: {item.error}</span></div>
{:else if item.data}
  {#if part === 'streams'}
    <!-- Without a stream the item shows nothing, as in PHP Organizr. -->
    {#if items.length > 0}
      <h2 class="org-item-title">
        Active Plex Streams<span aria-hidden="true">:</span>
        <span class="org-count">{items.length}</span>
      </h2>
      <ul class="org-cards" aria-label="Active Plex Streams">
        {#each items as stream, index (index)}
          {@const picture = pictureAddress(stream.nowPlayingImageURL)}
          <li class="org-stream">
            {#if picture}
              <img class="aspect-video w-full object-cover" src={picture} alt="" loading="lazy" />
            {/if}
            <progress
              class="org-stream-progress"
              value={stream.watched}
              max="100"
              aria-label="{stream.watched} percent watched"
            ></progress>
            <div class="flex items-center gap-2 px-2.5 pt-2">
              <span class="flex-1 truncate text-center font-medium text-white">
                {stream.nowPlayingTitle || stream.title}
              </span>
              <i
                class="oi oi-simpleline {stream.state === 'pause'
                  ? 'icon-control-pause'
                  : 'icon-control-play'} text-info"
                aria-hidden="true"
              ></i>
              <span class="sr-only">{stream.state === 'pause' ? 'Paused' : 'Playing'}</span>
            </div>
            <div class="flex items-center justify-between gap-2 px-2.5 pb-2 text-xs">
              <span class="truncate">{stream.nowPlayingBottom ?? ''}</span>
              {#if stream.user}
                <span class="shrink-0">
                  {stream.user}
                  <i class="oi oi-simpleline icon-user" aria-hidden="true"></i>
                </span>
              {/if}
            </div>
            <p class="truncate px-2.5 pb-2 text-xs opacity-60">{streamText(stream)}</p>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if part === 'recent'}
    {#if items.length === 0}
      <h2 class="org-item-title">Recently Added<span aria-hidden="true">:</span></h2>
      <p class="text-sm">Nothing was added.</p>
    {:else}
      <PlexPosters {items} label="Recently Added" />
    {/if}
  {:else}
    {#if playlist}
      {#if playlists.length > 1}
        <select
          class="select select-sm float-right w-auto"
          aria-label="Playlist"
          onchange={(event) => (chosen = event.currentTarget.value)}
        >
          {#each playlists as [name, list] (name)}
            <option value={name} selected={list === playlist}>{list.title}</option>
          {/each}
        </select>
      {/if}
      <PlexPosters items={videosOf(playlist)} label={playlist.title} />
    {:else}
      <h2 class="org-item-title">Playlists<span aria-hidden="true">:</span></h2>
      <p class="text-sm">There are no playlists to show.</p>
    {/if}
  {/if}
{/if}
