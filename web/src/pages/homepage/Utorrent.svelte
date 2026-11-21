<script lang="ts">
  // The uTorrent item: the torrents and how far each one is.
  import { itemData } from '../../lib/itemData.svelte';

  interface Torrent {
    Name?: string;
    /** The state in a word, for example "Downloading". */
    Status?: string;
    /** How far the torrent is, as text, for example "45.3%". */
    Percent?: string;
    Labels?: string;
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

  const item = itemData<Answer>('homepage/utorrent/queue', 'uTorrentRefresh');

  const content = $derived(item.data?.content || null);
  const torrents = $derived(content?.queueItems ?? []);

  function percentOf(torrent: Torrent): number {
    const percent = Number.parseFloat(torrent.Percent ?? '');
    return Number.isFinite(percent) ? Math.floor(Math.min(Math.max(percent, 0), 100)) : 0;
  }

  $effect(() => {
    oncount?.(torrents.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>uTorrent: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Torrents of uTorrent <span class="badge badge-info">{torrents.length}</span>
  </h2>
  {#if content === null}
    <p class="text-sm">uTorrent gave no answer. Look at its address and its login.</p>
  {:else if torrents.length === 0}
    <p class="org-queue-empty">Nothing in queue</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Torrents of uTorrent">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Label</th>
            <th scope="col">Done</th>
          </tr>
        </thead>
        <tbody>
          {#each torrents as torrent, index (index)}
            <tr>
              <td>{torrent.Name ?? ''}</td>
              <td>{torrent.Status ?? ''}</td>
              <td>{torrent.Labels ?? ''}</td>
              <td>
                <progress
                  class="progress w-24"
                  value={percentOf(torrent)}
                  max="100"
                  aria-label="{percentOf(torrent)} percent of {torrent.Name ?? 'the torrent'}"
                ></progress>
                {percentOf(torrent)} %
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}
