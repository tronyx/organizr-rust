<script lang="ts">
  // The SABnzbd item: the downloads in the queue, and the last ones that are done.
  import { api, failureText } from '../../lib/api';
  import { itemData } from '../../lib/itemData.svelte';
  import { session } from '../../lib/session.svelte';
  import QueueTabs from './QueueTabs.svelte';

  interface Slot {
    nzo_id: string;
    filename?: string;
    status?: string;
    percentage?: string | number;
    timeleft?: string;
    cat?: string;
  }

  interface Done {
    nzo_id?: string;
    name?: string;
    status?: string;
    size?: string;
  }

  interface Answer {
    content:
      | false
      | {
          queueItems?: { queue?: { paused?: boolean; speed?: string; slots?: Slot[] } };
          historyItems?: { history?: { slots?: Done[] } };
        };
  }

  /** How many downloads of the history are shown. */
  const HISTORY = 10;

  let {
    oncount,
  }: {
    /** Called with the number of entries, for the tab of the downloader item. */
    oncount?: (count: number) => void;
  } = $props();

  const item = itemData<Answer>('homepage/sabnzbd/queue', 'sabnzbdRefresh');

  const content = $derived(item.data?.content || null);
  const queue = $derived(content?.queueItems?.queue);
  const slots = $derived(queue?.slots ?? []);
  const done = $derived((content?.historyItems?.history?.slots ?? []).slice(0, HISTORY));

  let busy = $state(false);
  let error = $state<string | null>(null);

  /** Pauses or resumes the queue, or one download of it. */
  async function change(action: 'pause' | 'resume', target: string) {
    busy = true;
    error = null;
    try {
      await api('POST', `homepage/sabnzbd/queue/${action}`, { target });
      await item.reload();
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    oncount?.(slots.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>SABnzbd: {item.error}</span></div>
{:else if item.data}
  <h2 class="sr-only">Downloads of SABnzbd: {slots.length}</h2>
  {#if error}
    <div role="alert" class="alert alert-error mb-2"><span>{error}</span></div>
  {/if}
  {#if !content}
    <p class="text-sm">SABnzbd gave no answer. Look at its address and its API key.</p>
  {:else}
    <QueueTabs
      name="SABnzbd"
      paused={session.canManage && queue ? (queue.paused ?? false) : null}
      {busy}
      onpause={() => change(queue?.paused ? 'resume' : 'pause', 'main')}
    >
      {#snippet queuePart()}
        {#if slots.length === 0}
          <p class="org-queue-empty">Nothing in queue</p>
        {:else}
          <div class="overflow-x-auto">
            <table class="table-sm table" aria-label="Downloads of SABnzbd">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">State</th>
                  <th scope="col">Done</th>
                  <th scope="col">Time left</th>
                  {#if session.canManage}
                    <th scope="col"><span class="sr-only">Actions</span></th>
                  {/if}
                </tr>
              </thead>
              <tbody translate="no">
                {#each slots as slot (slot.nzo_id)}
                  {@const percent = Number(slot.percentage) || 0}
                  {@const paused = slot.status === 'Paused'}
                  <tr>
                    <td>{slot.filename ?? ''}</td>
                    <td>{slot.status ?? ''}</td>
                    <td>
                      <progress
                        class="progress w-24"
                        value={percent}
                        max="100"
                        aria-label="{percent} percent of {slot.filename ?? 'the download'}"
                      ></progress>
                      {percent} %
                    </td>
                    <td>{slot.timeleft ?? ''}</td>
                    {#if session.canManage}
                      <td>
                        <button
                          type="button"
                          class="btn btn-ghost btn-xs"
                          disabled={busy}
                          aria-label="{paused ? 'Resume' : 'Pause'} {slot.filename ??
                            'the download'}"
                          onclick={() => change(paused ? 'resume' : 'pause', slot.nzo_id)}
                        >
                          {paused ? 'Resume' : 'Pause'}
                        </button>
                      </td>
                    {/if}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/snippet}
      {#snippet historyPart()}
        {#if done.length === 0}
          <p class="org-queue-empty">Nothing in history</p>
        {:else}
          <ul class="text-sm" aria-label="Last downloads of SABnzbd">
            {#each done as one, index (one.nzo_id ?? index)}
              <li class="flex justify-between gap-2 px-4 py-2">
                <span class="truncate">{one.name ?? ''}</span>
                <span class="opacity-70">{one.status ?? ''} {one.size ?? ''}</span>
              </li>
            {/each}
          </ul>
        {/if}
      {/snippet}
    </QueueTabs>
  {/if}
{/if}
