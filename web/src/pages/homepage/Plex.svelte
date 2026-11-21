<script lang="ts">
  // A media server on the homepage: what plays now, what was added last, or the
  // playlists. Plex, Emby and Jellyfin show in the same way. Only Plex has playlists.
  import Dialog from '../../components/Dialog.svelte';
  import { itemData } from '../../lib/itemData.svelte';
  import {
    linkOf,
    type MediaService,
    pictureAddress,
    type PlexAnswer,
    type PlexItem,
    SERVICE_NAMES,
    streamIcon,
    streamLines,
    streamsSummary,
    streamText,
  } from '../../lib/plex';
  import MediaDetails from './MediaDetails.svelte';
  import PlexPosters from './PlexPosters.svelte';

  /** A playlist: its title, and its videos under the keys "0", "1" and so on. */
  type Playlist = { title: string } & Record<string, PlexItem | string>;

  interface PlaylistAnswer {
    content?: Record<string, Playlist>;
  }

  let {
    part,
    service = 'plex',
  }: { part: 'streams' | 'recent' | 'playlists'; service?: MediaService } = $props();

  // svelte-ignore state_referenced_locally
  const name = SERVICE_NAMES[service];

  // svelte-ignore state_referenced_locally
  const refresh = part === 'streams' ? 'homepageStreamRefresh' : 'homepageRecentRefresh';
  // svelte-ignore state_referenced_locally
  const item = itemData<PlexAnswer | PlaylistAnswer>(`homepage/${service}/${part}`, refresh);

  const items = $derived(
    part !== 'playlists' && Array.isArray(item.data?.content) ? item.data.content : [],
  );
  const playlists = $derived(
    part === 'playlists' && item.data?.content && !Array.isArray(item.data.content)
      ? Object.entries(item.data.content)
      : [],
  );

  /** The stream whose pop-up is open: how it plays, or its details. */
  let playing = $state.raw<PlexItem | null>(null);
  let detailsOf = $state.raw<PlexItem | null>(null);
  /** The wide pictures that were asked for anew, by the session of their stream. */
  let renewed = $state<Record<string, string>>({});

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
  <div role="alert" class="alert alert-error"><span>{name}: {item.error}</span></div>
{:else if item.data}
  {#if part === 'streams'}
    <!-- Without a stream the item shows nothing, as in PHP Organizr. -->
    {#if items.length > 0}
      <h2 class="org-item-title">
        Active {name} Streams<span aria-hidden="true">:</span>
        <span class="org-count">{items.length}</span>
        <span class="org-count" title={streamsSummary(items, service)}>
          <i class="oi oi-fontawesome fa-info" aria-hidden="true"></i>
          <span class="sr-only">{streamsSummary(items, service)}</span>
        </span>
      </h2>
      <ul class="org-cards" aria-label="Active {name} Streams">
        {#each items as stream, index (index)}
          {@const picture = renewed[stream.uid] ?? pictureAddress(stream.nowPlayingImageURL)}
          {@const link = linkOf(stream)}
          <li class="org-stream" title={streamText(stream)}>
            <div class="org-stream-picture">
              {#if picture}
                <img class="aspect-video w-full object-cover" src={picture} alt="" loading="lazy" />
              {:else}
                <div class="aspect-video w-full"></div>
              {/if}
              <!-- The buttons of PHP Organizr that show over the picture. -->
              <div class="org-stream-buttons">
                <button
                  type="button"
                  aria-label="How {stream.nowPlayingTitle || stream.title} plays"
                  onclick={() => (playing = stream)}
                >
                  <i class="oi oi-materialize mdi-server-network" aria-hidden="true"></i>
                </button>
                {#if stream.metadataKey}
                  <button
                    type="button"
                    aria-label="Details of {stream.title}"
                    onclick={() => (detailsOf = stream)}
                  >
                    <i class="oi oi-materialize mdi-information" aria-hidden="true"></i>
                  </button>
                {/if}
                {#if link}
                  <a
                    href={link}
                    target="_blank"
                    rel="noopener noreferrer"
                    aria-label="Open {stream.title} in {name}"
                  >
                    <i
                      class="oi oi-materialize mdi-{service === 'jellyfin' ? 'fish' : service}"
                      aria-hidden="true"
                    ></i>
                  </a>
                {/if}
                {#if stream.nowPlayingOriginalImage}
                  <button
                    type="button"
                    aria-label="Renew the picture of {stream.title}"
                    onclick={() => (renewed[stream.uid] = stream.nowPlayingOriginalImage ?? '')}
                  >
                    <i class="oi oi-materialize mdi-refresh" aria-hidden="true"></i>
                  </button>
                {/if}
              </div>
            </div>
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
              <span class="truncate">
                {#if streamIcon(stream)}
                  <i class="oi oi-simpleline {streamIcon(stream)} text-info" aria-hidden="true"></i>
                {/if}
                {stream.nowPlayingBottom ?? ''}
              </span>
              {#if stream.user}
                <span class="shrink-0">
                  {stream.user}
                  <i class="oi oi-simpleline icon-user" aria-hidden="true"></i>
                </span>
              {/if}
            </div>
            <!-- PHP Organizr shows how the stream plays in a pop-up; here it is the title of the card. -->
            <p class="sr-only">{streamText(stream)}</p>
          </li>
        {/each}
      </ul>
    {/if}
    {#if playing}
      {@const lines = streamLines(playing)}
      <Dialog title={playing.sessionType} onclose={() => (playing = null)}>
        <p class="text-sm opacity-70">
          <i class="oi oi-materialize mdi-network-upload" aria-hidden="true"></i>
          {playing.bandwidth ?? 0} kbps
        </p>
        <dl class="org-stream-info">
          {#if playing.bandwidthType}
            <dt>Network</dt>
            <dd class="uppercase">{playing.bandwidthType}</dd>
          {/if}
          {#if playing.userAddress}
            <dt>Address</dt>
            <dd>{playing.userAddress}</dd>
          {/if}
          <dt>Stream</dt>
          <dd>{lines.stream}</dd>
          {#if lines.video}
            <dt>Video</dt>
            <dd>{lines.video}</dd>
          {/if}
          <dt>Audio</dt>
          <dd>{lines.audio}</dd>
          <dt>Product</dt>
          <dd>{playing.userStream?.product ?? ''}</dd>
          <dt>Device</dt>
          <dd>{playing.userStream?.device ?? ''}</dd>
          <dt>Watched</dt>
          <dd>{playing.watched}%</dd>
        </dl>
        <div class="modal-action">
          <button type="button" class="btn btn-primary" onclick={() => (playing = null)}>
            Close
          </button>
        </div>
      </Dialog>
    {/if}
    {#if detailsOf}
      <MediaDetails
        item={detailsOf}
        detailsPath="homepage/{service}/metadata"
        serverName={name}
        onclose={() => (detailsOf = null)}
      />
    {/if}
  {:else if part === 'recent'}
    {#if items.length === 0}
      <h2 class="org-item-title">Recently Added<span aria-hidden="true">:</span></h2>
      <p class="text-sm">Nothing was added.</p>
    {:else}
      <PlexPosters {items} {service} label="Recently Added" byKind />
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
