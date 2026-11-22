<script lang="ts">
  // The homepage: the items that the server lists for this person, in their order.
  import BookmarksItem from './BookmarksItem.svelte';
  import { onMount } from 'svelte';

  import Notice from '../../components/Notice.svelte';
  import { api, failureText } from '../../lib/api';
  import type { CalendarOptions } from '../../lib/calendar';
  import { requestOptions } from '../../lib/requests';
  import type { HomepageItem } from '../../lib/homepage';
  import { donateOptions, jellyStatOptions, liveTvOptions } from '../../lib/statistics';
  import Adguard from './Adguard.svelte';
  import ArrQueue from './ArrQueue.svelte';
  import Calendar from './Calendar.svelte';
  import CustomHtml from './CustomHtml.svelte';
  import Deluge from './Deluge.svelte';
  import Donate from './Donate.svelte';
  import Downloaders from './Downloaders.svelte';
  import EmbyLiveTv from './EmbyLiveTv.svelte';
  import HealthChecks from './HealthChecks.svelte';
  import Indexers from './Indexers.svelte';
  import Jdownloader from './Jdownloader.svelte';
  import JellyStat from './JellyStat.svelte';
  import Monitorr from './Monitorr.svelte';
  import Netdata from './Netdata.svelte';
  import Octoprint from './Octoprint.svelte';
  import Overseerr from './Overseerr.svelte';
  import Pihole from './Pihole.svelte';
  import Plex from './Plex.svelte';
  import PromPage from './PromPage.svelte';
  import Nzbget from './Nzbget.svelte';
  import Qbittorrent from './Qbittorrent.svelte';
  import Rtorrent from './Rtorrent.svelte';
  import Sabnzbd from './Sabnzbd.svelte';
  import SpeedtestTracker from './SpeedtestTracker.svelte';
  import Tautulli from './Tautulli.svelte';
  import Transmission from './Transmission.svelte';
  import Unifi from './Unifi.svelte';
  import UptimeKuma from './UptimeKuma.svelte';
  import Utorrent from './Utorrent.svelte';
  import Weather from './Weather.svelte';

  let { title }: { title: string } = $props();

  /** The kinds of items that this interface can show. */
  const KNOWN = new Set([
    'customhtml',
    'downloader',
    'healthchecks',
    'uptimekuma',
    'calendar',
    'sonarrqueue',
    'radarrqueue',
    'sabnzbd',
    'qbittorrent',
    'nzbget',
    'transmission',
    'deluge',
    'rtorrent',
    'utorrent',
    'jdownloader',
    'pihole',
    'weather',
    'tautulli',
    'plexstreams',
    'plexrecent',
    'plexplaylists',
    'embystreams',
    'embyrecent',
    'jellyfinstreams',
    'jellyfinrecent',
    'overseerr',
    'ombi',
    'jackett',
    'prowlarr',
    'adguard',
    'netdata',
    'monitorr',
    'speedtest',
    'octoprint',
    'unifi',
    'prompage',
    'donate',
    'embylivetv',
    'jellystat',
    'bookmarks',
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

  /** The downloaders that share the downloader item. */
  function membersOf(item: HomepageItem): string[] {
    const members = item.options?.members;
    return Array.isArray(members)
      ? members.filter((member): member is string => typeof member === 'string')
      : [];
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
      <section class="org-item org-item-{item.kind}" data-item={item.key}>
        {#if item.kind === 'customhtml'}
          <CustomHtml id={item.id} trusted={item.options?.trusted === true} />
        {:else if item.kind === 'bookmarks'}
          <BookmarksItem />
        {:else if item.kind === 'downloader'}
          <Downloaders members={membersOf(item)} />
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
        {:else if item.kind === 'deluge'}
          <Deluge />
        {:else if item.kind === 'rtorrent'}
          <Rtorrent />
        {:else if item.kind === 'utorrent'}
          <Utorrent />
        {:else if item.kind === 'jdownloader'}
          <Jdownloader />
        {:else if item.kind === 'pihole'}
          <Pihole />
        {:else if item.kind === 'plexstreams'}
          <Plex part="streams" />
        {:else if item.kind === 'plexrecent'}
          <Plex part="recent" />
        {:else if item.kind === 'plexplaylists'}
          <Plex part="playlists" />
        {:else if item.kind === 'embystreams'}
          <Plex part="streams" service="emby" />
        {:else if item.kind === 'embyrecent'}
          <Plex part="recent" service="emby" />
        {:else if item.kind === 'jellyfinstreams'}
          <Plex part="streams" service="jellyfin" />
        {:else if item.kind === 'jellyfinrecent'}
          <Plex part="recent" service="jellyfin" />
        {:else if item.kind === 'overseerr'}
          <Overseerr />
        {:else if item.kind === 'ombi'}
          <Overseerr service="ombi" options={requestOptions(item.options)} />
        {:else if item.kind === 'tautulli'}
          <Tautulli />
        {:else if item.kind === 'jackett'}
          <Indexers service="jackett" blackHole={item.options?.blackHole === true} />
        {:else if item.kind === 'prowlarr'}
          <Indexers service="prowlarr" blackHole={item.options?.blackHole === true} />
        {:else if item.kind === 'adguard'}
          <Adguard />
        {:else if item.kind === 'netdata'}
          <Netdata />
        {:else if item.kind === 'monitorr'}
          <Monitorr />
        {:else if item.kind === 'speedtest'}
          <SpeedtestTracker />
        {:else if item.kind === 'octoprint'}
          <Octoprint />
        {:else if item.kind === 'unifi'}
          <Unifi />
        {:else if item.kind === 'prompage'}
          <PromPage />
        {:else if item.kind === 'donate'}
          <Donate options={donateOptions(item.options)} />
        {:else if item.kind === 'embylivetv'}
          <EmbyLiveTv options={liveTvOptions(item.options)} />
        {:else if item.kind === 'jellystat'}
          <JellyStat options={jellyStatOptions(item.options)} />
        {:else if item.kind === 'weather'}
          <Weather title={typeof item.options?.title === 'string' ? item.options.title : ''} />
        {/if}
      </section>
    {/each}
  </main>
{/if}
