<script lang="ts">
  // The PromPage item: each service of the status page, and whether it is up.
  import { itemData } from '../../lib/itemData.svelte';

  interface Service {
    name?: string;
    status?: boolean;
    /** The uptime in percent. */
    uptime?: number | null;
  }

  interface Answer {
    /** A list, or an object with the services as its values. */
    data: Service[] | Record<string, Service> | null;
    options: { title: string; titleToggle: boolean; compact: boolean; showUptime: boolean };
  }

  const item = itemData<Answer>('homepage/prompage/data', 'homepagePromPageRefresh');

  const services = $derived(Object.values(item.data?.data ?? {}));
  const options = $derived(item.data?.options);

  /** The uptime with two digits after the point at most. */
  function uptimeOf(service: Service): string | null {
    const uptime = Number(service.uptime);
    return service.uptime === null || service.uptime === undefined || !Number.isFinite(uptime)
      ? null
      : `${Math.round(uptime * 100) / 100}%`;
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>PromPage: {item.error}</span></div>
{:else if item.data}
  {#if options?.titleToggle}
    <h2 class="org-item-title">{options.title}<span aria-hidden="true">:</span></h2>
  {/if}
  {#if services.length === 0}
    <p class="text-sm">PromPage lists no services.</p>
  {/if}
  <ul translate="no" class="org-cards" aria-label={options?.title || 'PromPage'}>
    {#each services as service, index (index)}
      {@const up = service.status === true}
      {@const uptime = uptimeOf(service)}
      <li class="org-card" style:--org-state={up ? 'var(--color-success)' : 'var(--color-error)'}>
        <div class="org-card-row">
          <span class="flex min-w-0 flex-1 items-baseline gap-3">
            <span class="truncate">{service.name}</span>
            {#if options?.showUptime && uptime}
              <span class="text-xs opacity-60">{uptime}</span>
            {/if}
          </span>
          {#if options?.compact}
            <i
              class="oi oi-fontawesome {up
                ? 'fa-check-circle text-success'
                : 'fa-times-circle text-error'} text-xl"
              aria-hidden="true"
            ></i>
            <span class="sr-only">{up ? 'Up' : 'Down'}</span>
          {:else}
            <span class="badge {up ? 'badge-success' : 'badge-error'}">{up ? 'UP' : 'DOWN'}</span>
          {/if}
        </div>
      </li>
    {/each}
  </ul>
{/if}
