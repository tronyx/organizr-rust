<script lang="ts">
  // The NZBGet item: the downloads in the queue, and the last downloads of the history.
  import { itemData } from '../../lib/itemData.svelte';
  import QueueTabs from './QueueTabs.svelte';

  interface Download {
    NZBName?: string;
    Status?: string;
    FileSizeMB?: number;
    RemainingSizeMB?: number;
  }

  interface Done {
    Name?: string;
    Status?: string;
  }

  interface Answer {
    content: false | { queueItems?: { result?: Download[] }; historyItems?: { result?: Done[] } };
  }

  /** How many downloads of the history the item shows. */
  const HISTORY_SHOWN = 10;

  let {
    oncount,
  }: {
    /** Called with the number of entries, for the tab of the downloader item. */
    oncount?: (count: number) => void;
  } = $props();

  const item = itemData<Answer>('homepage/nzbget/queue', 'nzbgetRefresh');

  const content = $derived(item.data?.content || null);
  const queue = $derived(content?.queueItems?.result ?? []);
  const history = $derived((content?.historyItems?.result ?? []).slice(0, HISTORY_SHOWN));

  function percentOf(download: Download): number {
    const size = download.FileSizeMB ?? 0;
    if (size <= 0) {
      return 0;
    }
    const done = (size - (download.RemainingSizeMB ?? size)) / size;
    return Math.round(Math.min(Math.max(done, 0), 1) * 100);
  }

  /** A status of NZBGet in words: `SUCCESS/ALL` becomes "Success, all". */
  function statusText(status: string | undefined): string {
    const words = (status ?? '').toLowerCase().replaceAll('_', ' ').split('/');
    const text = words.join(', ');
    return text.charAt(0).toUpperCase() + text.slice(1);
  }

  $effect(() => {
    oncount?.(queue.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>NZBGet: {item.error}</span></div>
{:else if item.data}
  <h2 class="sr-only">Downloads of NZBGet: {queue.length}</h2>
  {#if content === null}
    <p class="text-sm">NZBGet gave no answer. Look at its address, user name and password.</p>
  {:else}
    <QueueTabs name="NZBGet">
      {#snippet queuePart()}
        {#if queue.length === 0}
          <p class="org-queue-empty">Nothing in queue</p>
        {:else}
          <div class="overflow-x-auto">
            <table class="table-sm table" aria-label="Queue of NZBGet">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">State</th>
                  <th scope="col">Done</th>
                </tr>
              </thead>
              <tbody translate="no">
                {#each queue as download, index (index)}
                  <tr>
                    <td>{download.NZBName ?? ''}</td>
                    <td>{statusText(download.Status)}</td>
                    <td>
                      <progress
                        class="progress w-24"
                        value={percentOf(download)}
                        max="100"
                        aria-label="{percentOf(download)} percent of {download.NZBName ??
                          'the download'}"
                      ></progress>
                      {percentOf(download)} %
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/snippet}
      {#snippet historyPart()}
        {#if history.length === 0}
          <p class="org-queue-empty">Nothing in history</p>
        {:else}
          <div class="overflow-x-auto">
            <table class="table-sm table" aria-label="History of NZBGet">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Result</th>
                </tr>
              </thead>
              <tbody translate="no">
                {#each history as done, index (index)}
                  <tr>
                    <td>{done.Name ?? ''}</td>
                    <td>{statusText(done.Status)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/snippet}
    </QueueTabs>
  {/if}
{/if}
