<script lang="ts">
  // The tabs QUEUE and HISTORY of a Usenet downloader in PHP Organizr, with the button
  // that pauses or resumes the whole queue next to them.
  import type { Snippet } from 'svelte';

  let {
    name,
    paused = null,
    busy = false,
    onpause,
    queuePart,
    historyPart,
  }: {
    /** The name of the downloader, for assistive tools. */
    name: string;
    /** Whether the queue is paused, or `null` when there is no button for it. */
    paused?: boolean | null;
    busy?: boolean;
    /** Pauses the queue, or resumes it. */
    onpause?: () => void;
    queuePart: Snippet;
    historyPart: Snippet;
  } = $props();

  let shown = $state<'queue' | 'history'>('queue');
  // svelte-ignore state_referenced_locally
  const id = `queue-tabs-${name.toLowerCase()}`;
</script>

<div class="org-queue-tabs">
  <div role="tablist" aria-label="Queue and history of {name}">
    <button
      type="button"
      role="tab"
      id="{id}-queue"
      aria-selected={shown === 'queue'}
      aria-controls="{id}-queue-panel"
      onclick={() => (shown = 'queue')}
    >
      <i class="oi oi-themify ti-download" aria-hidden="true"></i>
      <span>Queue</span>
    </button>
    <button
      type="button"
      role="tab"
      id="{id}-history"
      aria-selected={shown === 'history'}
      aria-controls="{id}-history-panel"
      onclick={() => (shown = 'history')}
    >
      <i class="oi oi-themify ti-time" aria-hidden="true"></i>
      <span>History</span>
    </button>
  </div>
  {#if paused !== null && onpause}
    <button
      type="button"
      class="org-queue-pause"
      disabled={busy}
      aria-label={paused ? `Resume all of ${name}` : `Pause all of ${name}`}
      onclick={onpause}
    >
      <i class="oi oi-fontawesome {paused ? 'fa-play' : 'fa-pause'}" aria-hidden="true"></i>
    </button>
  {/if}
</div>
<div role="tabpanel" id="{id}-queue-panel" aria-labelledby="{id}-queue" hidden={shown !== 'queue'}>
  {@render queuePart()}
</div>
<div
  role="tabpanel"
  id="{id}-history-panel"
  aria-labelledby="{id}-history"
  hidden={shown !== 'history'}
>
  {@render historyPart()}
</div>
