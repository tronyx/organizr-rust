<script lang="ts">
  // The Monitorr item: each service that Monitorr watches, and whether it is up.
  import { itemData } from '../../lib/itemData.svelte';
  import { safeLink } from '../../lib/monitors';

  interface Service {
    /** `true` for online, `false` for offline, and `unresponsive`. */
    status: boolean | string;
    sort?: number;
    /** A `data:` address with the picture, or a path when the picture could not be loaded. */
    image?: string;
    link?: string;
  }

  interface Answer {
    services?: Record<string, Service>;
    options: { title: string; titleToggle: boolean; compact: boolean };
  }

  const item = itemData<Answer>('homepage/monitorr/data', 'homepageMonitorrRefresh');

  const services = $derived(Object.entries(item.data?.services ?? {}));
  const options = $derived(item.data?.options);

  const STATES = {
    online: {
      colour: 'var(--color-success)',
      tone: 'success',
      icon: 'fa-check-circle',
      word: 'Online',
    },
    unresponsive: {
      colour: 'var(--color-warning)',
      tone: 'warning',
      icon: 'fa-times-circle',
      word: 'Unresponsive',
    },
    offline: {
      colour: 'var(--color-error)',
      tone: 'error',
      icon: 'fa-times-circle',
      word: 'Offline',
    },
  };

  function stateOf(service: Service) {
    if (service.status === true) {
      return STATES.online;
    }
    return service.status === 'unresponsive' ? STATES.unresponsive : STATES.offline;
  }

  /** The picture of a service, when the server sent one with its answer. */
  function pictureOf(service: Service): string | null {
    return service.image?.startsWith('data:image/') ? service.image : null;
  }
</script>

{#snippet name(text: string, service: Service, classes: string)}
  {@const link = safeLink(service.link)}
  {#if link}
    <a class={classes} href={link} target="_blank" rel="noopener noreferrer">{text}</a>
  {:else}
    <span class={classes}>{text}</span>
  {/if}
{/snippet}

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Monitorr: {item.error}</span></div>
{:else if item.data}
  <div>
    {#if options?.titleToggle}
      <h2 class="org-item-title">
        {options.title}<span aria-hidden="true">:</span>
        <span class="org-count">{services.length}</span>
      </h2>
    {/if}
    {#if services.length === 0}
      <p class="text-sm">Monitorr named no service. Look at its address.</p>
    {:else if options?.compact}
      <ul translate="no" class="org-cards" aria-label={options.title || 'Monitorr'}>
        {#each services as [text, service] (text)}
          {@const state = stateOf(service)}
          {@const picture = pictureOf(service)}
          <li class="org-card org-tone-{state.tone}">
            <div class="org-card-row">
              {#if picture}
                <img src={picture} alt="" />
              {/if}
              {@render name(text, service, 'min-w-0 flex-1 truncate')}
              <i class="oi oi-fontawesome {state.icon} org-card-icon text-xl" aria-hidden="true"
              ></i>
              <span class="sr-only">{state.word}</span>
            </div>
          </li>
        {/each}
      </ul>
    {:else}
      <ul
        class="grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6"
        aria-label={options?.title || 'Monitorr'}
      >
        {#each services as [text, service] (text)}
          {@const state = stateOf(service)}
          {@const picture = pictureOf(service)}
          <li class="flex flex-col items-center gap-3 bg-[#2d2c2c] p-4 text-center text-white">
            {@render name(text, service, 'max-w-full truncate text-lg')}
            {#if picture}
              <img class="h-20 w-20 object-contain" src={picture} alt="" />
            {/if}
            <span
              class="rounded px-4 py-1.5 text-xs font-semibold text-white uppercase"
              style:background={state.colour}
            >
              {state.word}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}
