<script lang="ts">
  // The rTorrent item: the torrents and how far each one is.
  import { itemData } from '../../lib/itemData.svelte';

  interface Torrent {
    name?: string;
    /** The state in a word, for example "Downloading". */
    status?: string;
    /** The size in bytes. rTorrent sends its numbers as text. */
    size?: string | number;
    /** The bytes that are complete. */
    downloaded?: string | number;
    label?: string;
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

  const item = itemData<Answer>('homepage/rtorrent/queue', 'rTorrentRefresh');

  const content = $derived(item.data?.content || null);
  const torrents = $derived(content?.queueItems ?? []);

  function percentOf(torrent: Torrent): number {
    const size = Number(torrent.size);
    const done = Number(torrent.downloaded);
    if (!(size > 0) || !(done > 0)) {
      return 0;
    }
    return Math.floor(Math.min(done / size, 1) * 100);
  }

  $effect(() => {
    oncount?.(torrents.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>rTorrent: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Torrents of rTorrent <span class="badge badge-info">{torrents.length}</span>
  </h2>
  {#if content === null}
    <p class="text-sm">rTorrent gave no answer. Look at its address and its login.</p>
  {:else if torrents.length === 0}
    <p class="org-queue-empty">Nothing in queue</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Torrents of rTorrent">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Label</th>
            <th scope="col">Done</th>
          </tr>
        </thead>
        <tbody translate="no">
          {#each torrents as torrent, index (index)}
            <tr>
              <td>{torrent.name ?? ''}</td>
              <td>{torrent.status ?? ''}</td>
              <td>{torrent.label ?? ''}</td>
              <td>
                <progress
                  class="progress w-24"
                  value={percentOf(torrent)}
                  max="100"
                  aria-label="{percentOf(torrent)} percent of {torrent.name ?? 'the torrent'}"
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
