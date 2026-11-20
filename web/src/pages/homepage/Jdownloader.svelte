<script lang="ts">
  // The JDownloader item: the packages that download, and the packages that wait in
  // the link grabber.
  import { itemData } from '../../lib/itemData.svelte';

  interface Package {
    name?: string;
    /** The speed as text. Nothing when the package does not download. */
    speed?: string | null;
    size?: string | null;
    /** The time that is left, as text. */
    eta?: string | null;
    /** From 0 to 100, as text. */
    percentage?: string | number | null;
    enabled?: boolean | null;
  }

  interface Content {
    queueItems: Package[];
    grabberItems: Package[];
    encryptedItems: Package[];
    offlineItems: Package[];
    /** The state of the downloads, whether the grabber collects, and whether an update waits. */
    $status?: [string | null, boolean | null, boolean | null];
  }

  interface Answer {
    content: false | Content;
  }

  interface Row {
    name: string;
    state: string;
    size: string;
    timeLeft: string;
    percent: number;
  }

  /** The states of the downloads of JDownloader in words. */
  const STATES: Record<string, string> = {
    RUNNING: 'Running',
    PAUSE: 'Paused',
    STOPPED_STATE: 'Stopped',
    IDLE: 'Stopped',
  };

  let {
    oncount,
  }: {
    /** Called with the number of entries, for the tab of the downloader item. */
    oncount?: (count: number) => void;
  } = $props();

  const item = itemData<Answer>('homepage/jdownloader/queue', 'jdownloaderRefresh');

  const content = $derived(item.data?.content || null);

  function percentOf(entry: Package): number {
    const percent = Number(entry.percentage);
    return Number.isFinite(percent) ? Math.floor(Math.min(Math.max(percent, 0), 100)) : 0;
  }

  /** The state of a package that downloads, as PHP Organizr names it. */
  function stateOf(entry: Package): string {
    if (entry.enabled == null) {
      return 'Disabled';
    }
    if (entry.eta == null && percentOf(entry) === 100) {
      return 'Completed';
    }
    return entry.speed ?? 'Stopped';
  }

  /** The packages of the link grabber, which have only a name and a state. */
  function waiting(entries: Package[] | undefined, state: string): Row[] {
    return (entries ?? []).map((entry) => ({
      name: entry.name ?? '',
      state,
      size: '',
      timeLeft: '',
      percent: 0,
    }));
  }

  const rows = $derived<Row[]>(
    content === null
      ? []
      : [
          ...(content.queueItems ?? []).map((entry) => ({
            name: entry.name ?? '',
            state: stateOf(entry),
            size: entry.size ?? '',
            timeLeft: entry.eta ?? '',
            percent: percentOf(entry),
          })),
          ...waiting(content.grabberItems, 'Online'),
          ...waiting(content.encryptedItems, 'Encrypted'),
          ...waiting(content.offlineItems, 'Offline'),
        ],
  );

  const state = $derived(content?.$status?.[0] ?? '');

  $effect(() => {
    oncount?.(rows.length);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>JDownloader: {item.error}</span></div>
{:else if item.data}
  <h2 class="mb-2 text-lg font-semibold">
    Packages of JDownloader <span class="badge badge-info">{rows.length}</span>
  </h2>
  {#if content === null}
    <p class="text-sm">JDownloader gave no answer. Look at its address and its login.</p>
  {:else if rows.length === 0}
    <p class="text-sm">There are no packages to show.</p>
  {:else}
    {#if state}
      <p class="mb-2 text-sm">Downloads: {STATES[state] ?? state}</p>
    {/if}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Packages of JDownloader">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Size</th>
            <th scope="col">Time left</th>
            <th scope="col">Done</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row, index (index)}
            <tr>
              <td>{row.name}</td>
              <td>{row.state}</td>
              <td>{row.size}</td>
              <td>{row.timeLeft}</td>
              <td>
                <progress
                  class="progress w-24"
                  value={row.percent}
                  max="100"
                  aria-label="{row.percent} percent of {row.name || 'the package'}"
                ></progress>
                {row.percent} %
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}
