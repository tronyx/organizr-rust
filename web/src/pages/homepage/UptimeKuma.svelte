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
  <!-- The ids and classes of PHP Organizr, for custom CSS (D-110). -->
  <div id="allUptimeKuma">
    {#if options?.titleToggle}
      <h2 class="org-item-title">{options.title}<span aria-hidden="true">:</span></h2>
    {/if}
    <ul
      translate="no"
      class="org-cards uptimeKumaCards"
      aria-label={options?.title || 'Uptime Kuma'}
    >
      {#each monitors as monitor, index (index)}
        {@const link = linkOf(monitor)}
        <li class="org-card col-xl-2 col-lg-3 col-md-4 col-sm-6 col-xs-12">
          <span class="left-health {monitor.status ? 'bg-success' : 'bg-danger'}" aria-hidden="true"
          ></span>
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
                ? 'fa-check-circle text-success'
                : 'fa-times-circle text-danger'} text-xl"
              aria-hidden="true"
            ></i>
            <span class="sr-only">{monitor.status ? 'Online' : 'Offline'}</span>
          </div>
        </li>
      {/each}
    </ul>
  </div>
{/if}
