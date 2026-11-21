<script lang="ts">
  // The Deluge item: the torrents and how far each one is.
  import { itemData } from '../../lib/itemData.svelte';
  import { durationText } from '../../lib/time';

  interface Torrent {
    name?: string;
    state?: string;
    /** From 0 to 100. */
    progress?: number;
    /** Seconds until the torrent is complete. Zero means "not known". */
    eta?: number;
    /** What the tracker said last. Empty when the settings leave it out. */
    tracker_status?: string;
  }

  interface Answer {
    content: false | { queueItems: Torrent[] };
  }

  let {
    oncount,
  }: {
    /** Called with the number of entries, for the tab of the downloader item. */
    oncount?: (count: number) => void;
  } = $props();

  const item = itemData<Answer>('homepage/deluge/queue', 'delugeRefresh');

  const content = $derived(item.data?.content || null);
  const torrents = $derived(content?.queueItems ?? []);

  function percentOf(torrent: Torrent): number {
    return Math.floor(Math.min(Math.max(Number(torrent.progress) || 0, 0), 100));
  }

  function timeLeft(torrent: Torrent): string {
    const eta = Number(torrent.eta) || 0;
    return percentOf(torrent) === 100 || eta <= 0 ? '' : durationText(eta);
  }

  $effect(() => {
    oncount?.(torrents.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Deluge: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Torrents of Deluge <span class="badge badge-info">{torrents.length}</span>
  </h2>
  {#if content === null}
    <p class="text-sm">Deluge gave no answer. Look at its address.</p>
  {:else if torrents.length === 0}
    <p class="org-queue-empty">Nothing in queue</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Torrents of Deluge">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Done</th>
            <th scope="col">Time left</th>
          </tr>
        </thead>
        <tbody translate="no">
          {#each torrents as torrent, index (index)}
            <tr>
              <td>
                {torrent.name ?? ''}
                {#if torrent.tracker_status}
                  <div class="text-xs opacity-70">{torrent.tracker_status}</div>
                {/if}
              </td>
              <td>{torrent.state ?? ''}</td>
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
