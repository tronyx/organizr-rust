<script lang="ts">
  // The Uptime Kuma item: each monitor, and whether it is up.
  import { itemData } from '../../lib/itemData.svelte';

  interface Monitor {
    name?: string;
    url?: string;
    type?: string;
    status: boolean;
    latency: number | null;
  }

  interface Answer {
    data: Monitor[];
    options: { title: string; titleToggle: boolean; compact: boolean; showLatency: boolean };
  }

  const item = itemData<Answer>('homepage/kuma/data', 'homepageUptimeKumaRefresh');

  const monitors = $derived(item.data?.data ?? []);
  const options = $derived(item.data?.options);

  /** The address of a monitor, when it is one that a browser can open. */
  function linkOf(monitor: Monitor): string | null {
    return monitor.url && /^https?:\/\//.test(monitor.url) ? monitor.url : null;
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Uptime Kuma: {item.error}</span></div>
{:else if item.data}
  {#if options?.titleToggle}
    <h2 class="mb-2 text-lg font-semibold">{options.title}</h2>
  {/if}
  <ul
    class="grid gap-2 {options?.compact
      ? 'sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6'
      : 'grid-cols-2 sm:grid-cols-3 lg:grid-cols-6'}"
    aria-label={options?.title || 'Uptime Kuma'}
  >
    {#each monitors as monitor, index (index)}
      {@const link = linkOf(monitor)}
      <li class="card bg-base-200">
        <div class="card-body gap-1 p-3 {options?.compact ? '' : 'items-center text-center'}">
          <span class="font-medium">
            {#if link}
              <a class="link link-hover" href={link} target="_blank" rel="noopener noreferrer">
                {monitor.name}
              </a>
            {:else}
              {monitor.name}
            {/if}
          </span>
          {#if options?.showLatency && monitor.latency !== null}
            <span class="text-sm opacity-70">{monitor.latency} ms</span>
          {/if}
          <span class="badge {monitor.status ? 'badge-success' : 'badge-error'}">
            {monitor.status ? 'Online' : 'Offline'}
          </span>
        </div>
      </li>
    {/each}
  </ul>
{/if}
