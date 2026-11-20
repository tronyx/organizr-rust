<script lang="ts">
  // The Transmission item: the torrents and how far each one is.
  import { itemData } from '../../lib/itemData.svelte';
  import { durationText } from '../../lib/time';

  interface Torrent {
    name?: string;
    /** The number that Transmission gives a state. */
    status?: number;
    /** From 0 to 1. */
    percentDone?: number;
    /** Seconds until the torrent is complete. A negative number means "not known". */
    eta?: number;
    errorString?: string;
  }

  interface Answer {
    content: false | { queueItems: Torrent[] };
  }

  /** The states of Transmission in words, by their numbers. */
  const STATES = [
    'Stopped',
    'Waits for a check',
    'Checking',
    'In the queue',
    'Downloading',
    'In the queue to upload',
    'Uploading',
  ];

  let {
    oncount,
  }: {
    /** Called with the number of entries, for the tab of the downloader item. */
    oncount?: (count: number) => void;
  } = $props();

  const item = itemData<Answer>('homepage/transmission/queue', 'transmissionRefresh');

  const content = $derived(item.data?.content || null);
  const torrents = $derived(content?.queueItems ?? []);

  function percentOf(torrent: Torrent): number {
    return Math.round(Math.min(Math.max(torrent.percentDone ?? 0, 0), 1) * 100);
  }

  function stateOf(torrent: Torrent): string {
    if (torrent.errorString) {
      return `Error: ${torrent.errorString}`;
    }
    return STATES[torrent.status ?? -1] ?? String(torrent.status ?? '');
  }

  function timeLeft(torrent: Torrent): string {
    const eta = torrent.eta ?? -1;
    return percentOf(torrent) === 100 || eta < 0 ? '' : durationText(eta);
  }

  $effect(() => {
    oncount?.(torrents.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Transmission: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Torrents of Transmission <span class="badge badge-info">{torrents.length}</span>
  </h2>
  {#if content === null}
    <p class="text-sm">Transmission gave no answer. Look at its address.</p>
  {:else if torrents.length === 0}
    <p class="text-sm">There are no torrents to show.</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Torrents of Transmission">
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
              <td>{stateOf(torrent)}</td>
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
