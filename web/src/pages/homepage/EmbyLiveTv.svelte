<script lang="ts">
  // The live television item of Emby: the numbers of the timers and of the recordings,
  // the recordings to come, and the recordings that are made.
  import { itemData } from '../../lib/itemData.svelte';
  import { type LiveTvOptions, momentText } from '../../lib/statistics';

  let { options }: { options: LiveTvOptions } = $props();

  interface Numbers {
    activeTimers?: number;
    seriesTimers?: number;
    todaysRecordings?: number;
    totalRecordings?: number;
  }

  interface Recording {
    date: string;
    name?: string | null;
    seriesName?: string | null;
    episodeTitle?: string | null;
    channelName?: string | null;
    userName?: string | null;
    duration?: string | null;
    /** `Scheduled`, `Recording` or `Completed`. */
    status?: string | null;
  }

  interface Lists {
    scheduledRecordings?: Recording[];
    completedRecordings?: Recording[];
  }

  const REFRESH = 'homepageEmbyLiveTVTrackerRefresh';
  // svelte-ignore state_referenced_locally
  const numbers = itemData<Numbers>(
    'homepage/embyLiveTVTracker/stats',
    REFRESH,
    options.refreshMinutes,
  );
  // svelte-ignore state_referenced_locally
  const lists = itemData<Lists>(
    'homepage/embyLiveTVTracker/activity',
    REFRESH,
    options.refreshMinutes,
  );

  /** The numbers of the item. The compact view shows the timers only. */
  const cards = $derived(
    [
      { title: 'Active Timers', value: numbers.data?.activeTimers },
      { title: 'Series Timers', value: numbers.data?.seriesTimers },
      { title: "Today's Recordings", value: numbers.data?.todaysRecordings },
      { title: 'Total Recordings', value: numbers.data?.totalRecordings },
    ].slice(0, options.compact ? 2 : 4),
  );
  const scheduled = $derived(lists.data?.scheduledRecordings ?? []);
  const completed = $derived(lists.data?.completedRecordings ?? []);

  /** The colour of the state of a recording. */
  function badgeOf(recording: Recording): string {
    const status = (recording.status ?? '').toLowerCase();
    if (status === 'recording') {
      return 'badge-error';
    }
    return status === 'completed' ? 'badge-success' : 'badge-info';
  }
</script>

<h2 class="org-item-title">Emby LiveTV Tracker<span aria-hidden="true">:</span></h2>
{#if numbers.error}
  <div role="alert" class="alert alert-error"><span>Emby live TV: {numbers.error}</span></div>
{:else if numbers.data}
  <div class="stats stats-vertical sm:stats-horizontal bg-base-200 mb-2 shadow">
    {#each cards as card (card.title)}
      <div class="stat">
        <div class="stat-title">{card.title}</div>
        <div class="stat-value text-2xl">{card.value ?? 0}</div>
      </div>
    {/each}
  </div>
{/if}
{#if lists.error}
  <div role="alert" class="alert alert-error"><span>Emby live TV: {lists.error}</span></div>
{:else if lists.data}
  <h3 class="mt-2 text-sm font-medium">Scheduled recordings</h3>
  {#if scheduled.length === 0}
    <p class="text-sm">No scheduled recordings</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Scheduled recordings">
        <thead>
          <tr>
            <th scope="col">Date</th>
            <th scope="col">Series</th>
            {#if options.showSeriesInfo}<th scope="col">Episode title</th>{/if}
            <th scope="col">Channel</th>
            {#if options.showUserInfo}<th scope="col">User</th>{/if}
            {#if options.showDuration}<th scope="col">Duration</th>{/if}
            <th scope="col">Status</th>
          </tr>
        </thead>
        <tbody translate="no">
          {#each scheduled as recording, index (index)}
            <tr>
              <td class="whitespace-nowrap">{momentText(recording.date)}</td>
              <td>{recording.seriesName || recording.name || '-'}</td>
              {#if options.showSeriesInfo}
                <td>{recording.episodeTitle || recording.name || '-'}</td>
              {/if}
              <td>{recording.channelName || '-'}</td>
              {#if options.showUserInfo}<td>{recording.userName || '-'}</td>{/if}
              {#if options.showDuration}<td>{recording.duration || '-'}</td>{/if}
              <td>
                <span class="badge badge-sm {badgeOf(recording)}">
                  {recording.status || 'Scheduled'}
                </span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  {#if options.showCompleted}
    <h3 class="mt-2 text-sm font-medium">Completed recordings</h3>
    {#if completed.length === 0}
      <p class="text-sm">No completed recordings</p>
    {:else}
      <div class="overflow-x-auto">
        <table class="table-sm table" aria-label="Completed recordings">
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th scope="col">Recording</th>
              {#if options.showSeriesInfo}<th scope="col">Series</th>{/if}
              {#if options.showDuration}<th scope="col">Duration</th>{/if}
            </tr>
          </thead>
          <tbody translate="no">
            {#each completed as recording, index (index)}
              <tr>
                <td class="whitespace-nowrap">{momentText(recording.date)}</td>
                <td>{recording.name || recording.seriesName || '-'}</td>
                {#if options.showSeriesInfo}<td>{recording.seriesName || '-'}</td>{/if}
                {#if options.showDuration}<td>{recording.duration || '-'}</td>{/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
{/if}
