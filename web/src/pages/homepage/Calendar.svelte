<script lang="ts">
  // The calendar item: the episodes, films and albums of the days around today.
  import Dialog from '../../components/Dialog.svelte';
  import {
    type CalendarEvent,
    type CalendarOptions,
    eventsByDay,
    KINDS,
    longDay,
    monthGrid,
    monthName,
    moveMonth,
    shownEvents,
    stateOf,
    STATES,
    timeOf,
    today,
    weekDays,
  } from '../../lib/calendar';
  import { itemData } from '../../lib/itemData.svelte';

  interface Answer {
    events: CalendarEvent[];
  }

  let { options }: { options: CalendarOptions } = $props();

  const item = itemData<Answer>('homepage/calendar', 'calendarRefresh');

  const now = today();
  let year = $state(Number(now.slice(0, 4)));
  let month = $state(Number(now.slice(5, 7)));
  // svelte-ignore state_referenced_locally
  let view = $state<'month' | 'list'>(options.defaultView === 'month' ? 'month' : 'list');
  let hiddenKinds = $state<string[]>([]);
  let hiddenStates = $state<string[]>([]);
  let open = $state<CalendarEvent | null>(null);

  const all = $derived(item.data?.events ?? []);
  const kinds = $derived(
    Object.keys(KINDS).filter((kind) => all.some((one) => one.imagetypeFilter === kind)),
  );
  const states = $derived(
    Object.keys(STATES).filter((state) => all.some((one) => one.downloadFilter === state)),
  );
  const byDay = $derived(
    eventsByDay(shownEvents(all, new Set(hiddenKinds), new Set(hiddenStates))),
  );
  const weeks = $derived(monthGrid(year, month, options.firstDay));
  /** The days of the shown month that have events, for the list. */
  const listDays = $derived(
    weeks
      .flat()
      .filter((day) => day.inMonth && byDay.has(day.day))
      .map((day) => day.day),
  );

  function move(step: number) {
    ({ year, month } = moveMonth(year, month, step));
  }

  function toToday() {
    year = Number(now.slice(0, 4));
    month = Number(now.slice(5, 7));
  }

  function toggle(list: string[], value: string): string[] {
    return list.includes(value) ? list.filter((one) => one !== value) : [...list, value];
  }

  /** The address of the picture of an event, or nothing for an event without one. */
  function pictureOf(event: CalendarEvent): string | null {
    const image = event.details.image;
    return image && !image.endsWith('/no-np.png') ? image : null;
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Calendar: {item.error}</span></div>
{:else if item.data}
  <div class="mb-2 flex flex-wrap items-center gap-2">
    <h2 class="mr-auto text-lg font-semibold" aria-live="polite">{monthName(year, month)}</h2>
    <div class="join">
      <button type="button" class="btn btn-sm join-item" onclick={() => move(-1)}> Earlier </button>
      <button type="button" class="btn btn-sm join-item" onclick={toToday}>Today</button>
      <button type="button" class="btn btn-sm join-item" onclick={() => move(1)}>Later</button>
    </div>
    <div class="join" role="group" aria-label="View of the calendar">
      <button
        type="button"
        class="btn btn-sm join-item"
        class:btn-active={view === 'month'}
        aria-pressed={view === 'month'}
        onclick={() => (view = 'month')}
      >
        Month
      </button>
      <button
        type="button"
        class="btn btn-sm join-item"
        class:btn-active={view === 'list'}
        aria-pressed={view === 'list'}
        onclick={() => (view = 'list')}
      >
        List
      </button>
    </div>
  </div>

  {#if kinds.length + states.length > 0}
    <fieldset class="mb-2 flex flex-wrap gap-x-4 gap-y-1 text-sm">
      <legend class="sr-only">What the calendar shows</legend>
      {#each kinds as kind (kind)}
        <label class="flex items-center gap-1">
          <input
            type="checkbox"
            class="checkbox checkbox-xs"
            checked={!hiddenKinds.includes(kind)}
            onchange={() => (hiddenKinds = toggle(hiddenKinds, kind))}
          />
          {KINDS[kind]?.word}
        </label>
      {/each}
      {#each states as state (state)}
        <label class="flex items-center gap-1">
          <input
            type="checkbox"
            class="checkbox checkbox-xs"
            checked={!hiddenStates.includes(state)}
            onchange={() => (hiddenStates = toggle(hiddenStates, state))}
          />
          {STATES[state]?.word}
        </label>
      {/each}
    </fieldset>
  {/if}

  {#if view === 'month'}
    <table class="w-full table-fixed border-collapse text-sm" aria-label={monthName(year, month)}>
      <thead>
        <tr>
          {#each weekDays(options.firstDay) as name (name)}
            <th scope="col" class="p-1 text-left font-medium opacity-70">{name}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each weeks as week (week[0]?.day)}
          <tr>
            {#each week as day (day.day)}
              {@const events = byDay.get(day.day) ?? []}
              <td
                class="border-base-300 h-20 border p-1 align-top"
                class:opacity-40={!day.inMonth}
                class:bg-base-200={day.day === now}
                aria-current={day.day === now ? 'date' : undefined}
              >
                <div class="text-xs opacity-70">{day.number}</div>
                {#each events.slice(0, options.limit) as event (event.id)}
                  <button
                    type="button"
                    class="badge {stateOf(event)
                      .badge} mb-0.5 block h-auto w-full truncate text-left"
                    title="{event.title}: {stateOf(event).word}"
                    onclick={() => (open = event)}
                  >
                    {event.title}
                  </button>
                {/each}
                {#if events.length > options.limit}
                  <button type="button" class="link text-xs" onclick={() => (view = 'list')}>
                    {events.length - options.limit} more
                  </button>
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if listDays.length === 0}
    <p class="text-sm">The calendar has nothing for this month.</p>
  {:else}
    <div class="flex flex-col gap-2">
      {#each listDays as day (day)}
        <section aria-label={longDay(day)}>
          <h3 class="text-sm font-medium opacity-70">{longDay(day)}</h3>
          <ul>
            {#each byDay.get(day) ?? [] as event (event.id)}
              <li>
                <button
                  type="button"
                  class="hover:bg-base-200 flex w-full items-center gap-2 rounded p-1 text-left"
                  onclick={() => (open = event)}
                >
                  <span class="w-12 text-sm tabular-nums opacity-70">
                    {timeOf(event.start) ?? ''}
                  </span>
                  <span class="flex-1 truncate">
                    {event.details.topTitle}
                    {#if event.details.bottomTitle}
                      <span class="opacity-70">{event.details.bottomTitle}</span>
                    {/if}
                  </span>
                  <span class="badge badge-sm {stateOf(event).badge}">{stateOf(event).word}</span>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
{/if}

{#if open}
  {@const event = open}
  {@const picture = pictureOf(event)}
  <Dialog title={event.details.topTitle} onclose={() => (open = null)}>
    {#if event.details.bottomTitle}
      <p class="opacity-70">{event.details.bottomTitle}</p>
    {/if}
    {#if picture}
      <img class="my-2 max-h-64 w-full rounded object-cover" src={picture} alt="" />
    {/if}
    <p class="my-2 flex flex-wrap gap-1">
      <span class="badge {stateOf(event).badge}">{stateOf(event).word}</span>
      <span class="badge badge-outline">{longDay(event.start.slice(0, 10))}</span>
      {#each event.releaseType ?? [] as release (release)}
        <span class="badge badge-outline">{release}</span>
      {/each}
      {#if event.details.runtime}
        <span class="badge badge-outline">{event.details.runtime} minutes</span>
      {/if}
      {#if event.details.ratings}
        <span class="badge badge-outline">Rating {event.details.ratings}</span>
      {/if}
      {#each event.details.genres ?? [] as genre (genre)}
        <span class="badge badge-ghost">{genre}</span>
      {/each}
    </p>
    {#if event.details.overview}
      <p class="text-sm">{event.details.overview}</p>
    {/if}
    <div class="modal-action">
      {#if event.details.showLink && /^https?:\/\//.test(event.details.href)}
        <a class="btn" href={event.details.href} target="_blank" rel="noopener noreferrer">
          Open in the application
        </a>
      {/if}
      <button type="button" class="btn btn-primary" onclick={() => (open = null)}>Close</button>
    </div>
  </Dialog>
{/if}
