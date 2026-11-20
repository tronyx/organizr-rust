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
  import Overseerr from './Overseerr.svelte';
  import Pihole from './Pihole.svelte';
  import Plex from './Plex.svelte';
  import Nzbget from './Nzbget.svelte';
  import Qbittorrent from './Qbittorrent.svelte';
  import Sabnzbd from './Sabnzbd.svelte';
  import Tautulli from './Tautulli.svelte';
  import Transmission from './Transmission.svelte';
  import UptimeKuma from './UptimeKuma.svelte';
  import Weather from './Weather.svelte';

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
    'nzbget',
    'transmission',
    'pihole',
    'weather',
    'tautulli',
    'plexstreams',
    'plexrecent',
    'plexplaylists',
    'overseerr',
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
  <main class="flex flex-col gap-2.5 px-[25px] pt-2.5 pb-[15px]" aria-label={title}>
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
        {:else if item.kind === 'nzbget'}
          <Nzbget />
        {:else if item.kind === 'transmission'}
          <Transmission />
        {:else if item.kind === 'pihole'}
          <Pihole />
        {:else if item.kind === 'plexstreams'}
          <Plex part="streams" />
        {:else if item.kind === 'plexrecent'}
          <Plex part="recent" />
        {:else if item.kind === 'plexplaylists'}
          <Plex part="playlists" />
        {:else if item.kind === 'overseerr'}
          <Overseerr />
        {:else if item.kind === 'tautulli'}
          <Tautulli />
        {:else if item.kind === 'weather'}
          <Weather title={typeof item.options?.title === 'string' ? item.options.title : ''} />
        {/if}
      </section>
    {/each}
  </main>
{/if}
