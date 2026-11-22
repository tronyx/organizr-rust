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
  <div>
    {#if options?.titleToggle}
      <h2 class="org-item-title">{options.title}<span aria-hidden="true">:</span></h2>
    {/if}
    <ul translate="no" class="org-cards" aria-label={options?.title || 'Uptime Kuma'}>
      {#each monitors as monitor, index (index)}
        {@const link = linkOf(monitor)}
        <li class="org-card {monitor.status ? 'org-tone-success' : 'org-tone-error'}">
          <div class="org-card-row">
            <span class="flex min-w-0 flex-1 items-baseline gap-3">
              {#if link}
                <a class="truncate" href={link} target="_blank" rel="noopener noreferrer">
                  {monitor.name}
                </a>
              {:else}
                <span class="truncate">{monitor.name}</span>
              {/if}
              {#if options?.showLatency && monitor.latency !== null}
                <span class="text-xs opacity-60">{monitor.latency}ms</span>
              {/if}
            </span>
            <i
              class="oi oi-fontawesome {monitor.status
                ? 'fa-check-circle'
                : 'fa-times-circle'} org-card-icon text-xl"
              aria-hidden="true"
            ></i>
            <span class="sr-only">{monitor.status ? 'Online' : 'Offline'}</span>
          </div>
        </li>
      {/each}
    </ul>
  </div>
{/if}
