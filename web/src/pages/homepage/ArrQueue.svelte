<script lang="ts">
  // The download queue of Sonarr or Radarr.
  import { itemData } from '../../lib/itemData.svelte';

  interface QueueItem {
    title?: string | null;
    status?: string | null;
    size?: number | null;
    sizeleft?: number | null;
    timeleft?: string | null;
    series?: { title?: string | null } | null;
    episode?: {
      seasonNumber?: number | null;
      episodeNumber?: number | null;
      title?: string | null;
    } | null;
  }

  interface Answer {
    content: { queueItems: QueueItem[] };
  }

  let {
    application,
    oncount,
  }: {
    application: 'sonarr' | 'radarr';
    /** Called with the number of entries, for the tab of the downloader item. */
    oncount?: (count: number) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  const name = application === 'sonarr' ? 'Sonarr' : 'Radarr';
  // svelte-ignore state_referenced_locally
  const item = itemData<Answer>(`homepage/${application}/queue`, `homepage${name}QueueRefresh`);

  const downloads = $derived(item.data?.content.queueItems ?? []);

  function two(number: number | null | undefined): string {
    return String(number ?? 0).padStart(2, '0');
  }

  /** What is downloaded: the series with its episode, or the name of the release. */
  function nameOf(download: QueueItem): string {
    if (download.series?.title && download.episode) {
      const episode = download.episode;
      return `${download.series.title} S${two(episode.seasonNumber)}E${two(episode.episodeNumber)}`;
    }
    return download.title || 'Unknown';
  }

  /** How much of a download is there, in percent. */
  function percentOf(download: QueueItem): number {
    const size = download.size ?? 0;
    if (size <= 0) {
      return 0;
    }
    const done = (size - (download.sizeleft ?? 0)) / size;
    return Math.round(Math.min(Math.max(done, 0), 1) * 100);
  }

  $effect(() => {
    oncount?.(downloads.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>{name}: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Downloads of {name} <span class="badge badge-info">{downloads.length}</span>
  </h2>
  {#if downloads.length === 0}
    <p class="org-queue-empty">Nothing in queue</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Downloads of {name}">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Done</th>
            <th scope="col">Time left</th>
          </tr>
        </thead>
        <tbody>
          {#each downloads as download, index (index)}
            <tr>
              <td>{nameOf(download)}</td>
              <td>{download.status ?? ''}</td>
              <td>
                <progress
                  class="progress w-24"
                  value={percentOf(download)}
                  max="100"
                  aria-label="{percentOf(download)} percent of {nameOf(download)}"
                ></progress>
                {percentOf(download)} %
              </td>
              <td>{download.timeleft ?? ''}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}
