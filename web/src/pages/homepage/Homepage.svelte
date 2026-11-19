<script lang="ts">
  // The homepage: the items that the server lists for this person, in their order.
  import { onMount } from 'svelte';

  import Notice from '../../components/Notice.svelte';
  import { api, failureText } from '../../lib/api';
  import type { CalendarOptions } from '../../lib/calendar';
  import type { HomepageItem } from '../../lib/homepage';
  import ArrQueue from './ArrQueue.svelte';
  import Calendar from './Calendar.svelte';
  import CustomHtml from './CustomHtml.svelte';
  import HealthChecks from './HealthChecks.svelte';
  import Qbittorrent from './Qbittorrent.svelte';
  import Sabnzbd from './Sabnzbd.svelte';
  import UptimeKuma from './UptimeKuma.svelte';

  let { title }: { title: string } = $props();

  /** The kinds of items that this interface can show. */
  const KNOWN = new Set([
    'customhtml',
    'healthchecks',
    'uptimekuma',
    'calendar',
    'sonarrqueue',
    'radarrqueue',
    'sabnzbd',
    'qbittorrent',
  ]);

  /** How the calendar shows, from the options of its item. */
  function calendarOptions(item: HomepageItem): CalendarOptions {
    const options = item.options ?? {};
    const firstDay = Number(options.firstDay);
    const limit = Number(options.limit);
    return {
      defaultView: typeof options.defaultView === 'string' ? options.defaultView : 'month',
      firstDay: Number.isInteger(firstDay) && firstDay >= 0 && firstDay <= 6 ? firstDay : 1,
      limit: Number.isInteger(limit) && limit > 0 ? limit : 1000,
    };
  }

  let items = $state<HomepageItem[] | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const listed = (await api<HomepageItem[]>('GET', 'homepage/items')).data;
      items = listed.filter((item) => KNOWN.has(item.kind));
    } catch (failure) {
      error = failureText(failure);
    }
  });
</script>

{#if error}
  <Notice {title}>
    <div role="alert" class="alert alert-error"><span>{error}</span></div>
  </Notice>
{:else if items === null}
  <main class="grid min-h-[60vh] place-items-center" aria-busy="true">
    <span class="loading loading-spinner loading-lg" aria-label="The homepage is loading"></span>
  </main>
{:else if items.length === 0}
  <Notice {title}>
    <p>The homepage has no items for you.</p>
  </Notice>
{:else}
  <main class="flex flex-col gap-4 p-4" aria-label={title}>
    {#each items as item (item.key)}
      <section data-item={item.key}>
        {#if item.kind === 'customhtml'}
          <CustomHtml id={item.id} />
        {:else if item.kind === 'healthchecks'}
          <HealthChecks />
        {:else if item.kind === 'uptimekuma'}
          <UptimeKuma />
        {:else if item.kind === 'calendar'}
          <Calendar options={calendarOptions(item)} />
        {:else if item.kind === 'sonarrqueue'}
          <ArrQueue application="sonarr" />
        {:else if item.kind === 'radarrqueue'}
          <ArrQueue application="radarr" />
        {:else if item.kind === 'sabnzbd'}
          <Sabnzbd />
        {:else if item.kind === 'qbittorrent'}
          <Qbittorrent />
        {/if}
      </section>
    {/each}
  </main>
{/if}
