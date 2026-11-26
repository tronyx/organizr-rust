<script lang="ts">
  // The JellyStat item: the statistics of an Emby or a Jellyfin server, or the pages of
  // JellyStat in a frame.
  import { itemData } from '../../lib/itemData.svelte';
  import type { PlexItem } from '../../lib/plex';
  import {
    frameAddress,
    frameSandbox,
    type JellyStatOptions,
    jellyStatPoster,
  } from '../../lib/statistics';
  import { fromNow } from '../../lib/time';
  import PlexPosters from './PlexPosters.svelte';

  let { options }: { options: JellyStatOptions } = $props();

  interface Library {
    name?: string;
    type?: string;
    item_count?: number | string;
    season_count?: number | string;
    episode_count?: number | string;
  }

  interface Totals {
    total_libraries?: number;
    total_items?: number;
    total_seasons?: number;
    total_episodes?: number;
  }

  interface Played {
    id?: string | number | null;
    title?: string;
    play_count?: number;
    year?: string | null;
  }

  interface User {
    name?: string;
    play_count?: number;
    last_activity?: string | null;
  }

  interface Statistics {
    period?: string;
    libraries?: Library[];
    /** An empty list when JellyStat gave no libraries. */
    library_totals?: Totals | [];
    users?: User[];
    most_watched_movies?: Played[];
    most_watched_shows?: Played[];
    most_listened_music?: Played[];
  }

  // svelte-ignore state_referenced_locally
  const isFrame = options.mode === 'iframe';
  // svelte-ignore state_referenced_locally
  const item = isFrame
    ? null
    : itemData<Statistics>(
        'homepage/jellystat',
        'homepageJellyStatRefresh',
        options.refreshMinutes,
      );

  const frame = $derived(frameAddress(options.url));
  const totals = $derived.by((): Totals => {
    const sent = item?.data?.library_totals;
    return sent && !Array.isArray(sent) ? sent : {};
  });
  const libraries = $derived(options.showLibraries ? (item?.data?.libraries ?? []) : []);
  const users = $derived(
    options.showUsers ? (item?.data?.users ?? []).slice(0, options.maxItems) : [],
  );

  /** The items of a list in the form of the posters of the media servers. */
  function posters(list: Played[] | undefined, type: string): PlexItem[] {
    return (list ?? []).map((played) => {
      const id = String(played.id ?? '');
      const plays = `${played.play_count ?? 0} ${played.play_count === 1 ? 'play' : 'plays'}`;
      return {
        type,
        title: played.title ?? '',
        secondaryTitle: played.year ? `${played.year} · ${plays}` : plays,
        summary: '',
        uid: id,
        // The details come by the id of the item. A name is no id.
        metadataKey: /^[A-Za-z0-9-]+$/.test(id) ? id : undefined,
        imageURL: jellyStatPoster(options.url, played.id) ?? '',
        nowPlayingImageURL: '',
        watched: 0,
        state: '',
        sessionType: '',
        user: '',
        address: null,
      };
    });
  }

  const rows = $derived(
    options.showMostWatched
      ? [
          {
            title: 'Most Watched Movies',
            items: options.showMovies ? posters(item?.data?.most_watched_movies, 'movie') : [],
          },
          {
            title: 'Most Watched TV Shows',
            items: options.showShows ? posters(item?.data?.most_watched_shows, 'tv') : [],
          },
          {
            title: 'Most Listened Music',
            items: options.showMusic ? posters(item?.data?.most_listened_music, 'music') : [],
          },
        ].filter((row) => row.items.length > 0)
      : [],
  );
</script>

{#if isFrame}
  <h2 class="org-item-title">JellyStat<span aria-hidden="true">:</span></h2>
  {#if frame}
    <iframe
      class="w-full border-0"
      title="JellyStat"
      src={frame}
      height={options.frameHeight}
      scrolling={options.frameScrolling ? 'auto' : 'no'}
      sandbox={frameSandbox(frame)}
      referrerpolicy="no-referrer"
      loading="lazy"
    ></iframe>
  {:else}
    <p class="text-sm">The address of JellyStat is no web address.</p>
  {/if}
{:else if item?.error}
  <div role="alert" class="alert alert-error"><span>JellyStat: {item.error}</span></div>
{:else if item?.data}
  <h2 class="org-item-title">
    JellyStat<span aria-hidden="true">:</span>
    {#if item.data.period}<span class="org-count">{item.data.period}</span>{/if}
  </h2>
  {#if libraries.length > 0}
    <div class="stats stats-vertical sm:stats-horizontal bg-base-200 mb-2 shadow">
      <div class="stat">
        <div class="stat-title">Libraries</div>
        <div class="stat-value text-2xl">{totals.total_libraries ?? 0}</div>
      </div>
      <div class="stat">
        <div class="stat-title">Items</div>
        <div class="stat-value text-2xl">{totals.total_items ?? 0}</div>
      </div>
      <div class="stat">
        <div class="stat-title">Seasons</div>
        <div class="stat-value text-2xl">{totals.total_seasons ?? 0}</div>
      </div>
      <div class="stat">
        <div class="stat-title">Episodes</div>
        <div class="stat-value text-2xl">{totals.total_episodes ?? 0}</div>
      </div>
    </div>
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Libraries of JellyStat">
        <thead>
          <tr>
            <th scope="col">Library</th>
            <th scope="col">Type</th>
            <th scope="col">Items</th>
            <th scope="col">Seasons</th>
            <th scope="col">Episodes</th>
          </tr>
        </thead>
        <tbody translate="no">
          {#each libraries as library, index (index)}
            <tr>
              <td>{library.name ?? ''}</td>
              <td>{library.type ?? ''}</td>
              <td>{library.item_count ?? 0}</td>
              <td>{library.season_count ?? 0}</td>
              <td>{library.episode_count ?? 0}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  {#if users.length > 0}
    <h3 class="mt-2 text-sm font-medium">Active users</h3>
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Users of JellyStat">
        <thead>
          <tr>
            <th scope="col">User</th>
            <th scope="col">Plays</th>
            <th scope="col">Last activity</th>
          </tr>
        </thead>
        <tbody translate="no">
          {#each users as user, index (index)}
            <tr>
              <td>{user.name ?? ''}</td>
              <td>{user.play_count ?? 0}</td>
              <td>{fromNow(user.last_activity) ?? '-'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  {#each rows as row (row.title)}
    <PlexPosters
      items={row.items}
      label={row.title}
      detailsPath="homepage/jellystat/metadata"
      serverName="the media server"
    />
  {/each}
  {#if libraries.length === 0 && users.length === 0 && rows.length === 0}
    <p class="text-sm">JellyStat gave no statistics. Look at its address and its API key.</p>
  {/if}
{/if}
