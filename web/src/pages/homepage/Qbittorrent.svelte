<script lang="ts">
  // The qBittorrent item: the torrents and how far each one is.
  import { itemData } from '../../lib/itemData.svelte';
  import { durationText } from '../../lib/time';

  interface Torrent {
    name?: string;
    state?: string;
    /** From 0 to 1. */
    progress?: number;
    /** Seconds until the torrent is complete. qBittorrent sends 8640000 for "not known". */
    eta?: number;
  }

  interface Answer {
    content: { queueItems: Torrent[] };
  }

  /** What qBittorrent sends when it does not know how long a torrent takes. */
  const UNKNOWN_TIME = 8640000;

  /** The states of qBittorrent in words. */
  const STATES: Record<string, string> = {
    downloading: 'Downloading',
    forcedDL: 'Downloading',
    metaDL: 'Reading the torrent',
    stalledDL: 'Waiting for peers',
    queuedDL: 'In the queue',
    pausedDL: 'Paused',
    stoppedDL: 'Paused',
    checkingDL: 'Checking',
    checkingUP: 'Checking',
    uploading: 'Uploading',
    forcedUP: 'Uploading',
    stalledUP: 'Uploading',
    queuedUP: 'In the queue to upload',
    pausedUP: 'Complete',
    stoppedUP: 'Complete',
    error: 'Error',
    missingFiles: 'Files are missing',
  };

  const item = itemData<Answer>('homepage/qbittorrent/queue', 'qBittorrentRefresh');

  const torrents = $derived(item.data?.content.queueItems ?? []);

  function percentOf(torrent: Torrent): number {
    return Math.round(Math.min(Math.max(torrent.progress ?? 0, 0), 1) * 100);
  }

  function timeLeft(torrent: Torrent): string {
    const eta = torrent.eta ?? UNKNOWN_TIME;
    return percentOf(torrent) === 100 || eta >= UNKNOWN_TIME ? '' : durationText(eta);
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>qBittorrent: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Torrents of qBittorrent <span class="badge badge-info">{torrents.length}</span>
  </h2>
  {#if torrents.length === 0}
    <p class="text-sm">There are no torrents to show.</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Torrents of qBittorrent">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Done</th>
            <th scope="col">Time left</th>
          </tr>
        </thead>
        <tbody>
          {#each torrents as torrent, index (index)}
            <tr>
              <td>{torrent.name ?? ''}</td>
              <td>{STATES[torrent.state ?? ''] ?? torrent.state ?? ''}</td>
              <td>
                <progress
                  class="progress w-24"
                  value={percentOf(torrent)}
                  max="100"
                  aria-label="{percentOf(torrent)} percent of {torrent.name ?? 'the torrent'}"
                ></progress>
                {percentOf(torrent)} %
              </td>
              <td>{timeLeft(torrent)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}
