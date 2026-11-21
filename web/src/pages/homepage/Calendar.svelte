<script lang="ts">
  // The calendar item: the episodes, films, albums and calendar events of the days
  // around today.
  import Dialog from '../../components/Dialog.svelte';
  import {
    addDays,
    allEvents,
    type CalendarAnswer,
    type CalendarEvent,
    type CalendarOptions,
    type CalendarView,
    dayOf,
    eventsByDay,
    genresOf,
    KINDS,
    longDay,
    monthGrid,
    monthName,
    moveMonth,
    rangeTitle,
    shortDay,
    shortTime,
    shownEvents,
    stateColour,
    stateOf,
    STATES,
    timeOf,
    today,
    viewOf,
    weekDays,
    weekOf,
  } from '../../lib/calendar';
  import { itemData } from '../../lib/itemData.svelte';

  let { options }: { options: CalendarOptions } = $props();

  const item = itemData<CalendarAnswer>('homepage/calendar', 'calendarRefresh');

  const now = today();
  // The day that the views stand on: its month, its week, or the day itself.
  let cursor = $state(now);
  const year = $derived(Number(cursor.slice(0, 4)));
  const month = $derived(Number(cursor.slice(5, 7)));
  // svelte-ignore state_referenced_locally
  let view = $state<CalendarView>(viewOf(options.defaultView));
  let filtersOpen = $state(false);
  let hiddenKinds = $state<string[]>([]);
  let hiddenStates = $state<string[]>([]);
  let open = $state<CalendarEvent | null>(null);

  const all = $derived(allEvents(item.data));
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

  /** The days of the week view, or the one day of the day view. */
  const columns = $derived(view === 'day' ? [cursor] : weekOf(cursor, options.firstDay));
  const title = $derived(
    view === 'week' || view === 'day'
      ? rangeTitle(columns[0] ?? cursor, columns.at(-1) ?? cursor)
      : monthName(year, month),
  );

  /** Goes one month, one week or one day on or back, as the view asks. */
  function move(step: number) {
    if (view === 'week') {
      cursor = addDays(cursor, 7 * step);
    } else if (view === 'day') {
      cursor = addDays(cursor, step);
    } else {
      const moved = moveMonth(year, month, step);
      const day = `${moved.year}-${String(moved.month).padStart(2, '0')}-01`;
      // The month of today stands on today, so the week view opens on this week.
      cursor = day.slice(0, 7) === now.slice(0, 7) ? now : day;
    }
  }

  function toToday() {
    cursor = now;
  }

  const VIEWS: [CalendarView, string][] = [
    ['month', 'Month'],
    ['week', 'Week'],
    ['day', 'Day'],
    ['list', 'List'],
  ];

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
  <div class="org-calendar">
    <div class="org-calendar-bar">
      <div class="org-calendar-buttons">
        <button type="button" aria-label="Earlier" onclick={() => move(-1)}>
          <i class="oi oi-fontawesome fa-chevron-left" aria-hidden="true"></i>
        </button>
        <button type="button" aria-label="Later" onclick={() => move(1)}>
          <i class="oi oi-fontawesome fa-chevron-right" aria-hidden="true"></i>
        </button>
        <button type="button" disabled={cursor === now} onclick={toToday}>Today</button>
      </div>
      <h2 aria-live="polite">{title}</h2>
      <div class="org-calendar-buttons" role="group" aria-label="View of the calendar">
        <button type="button" onclick={() => void item.reload()}>Refresh</button>
        {#if kinds.length + states.length > 0}
          <button
            type="button"
            aria-expanded={filtersOpen}
            onclick={() => (filtersOpen = !filtersOpen)}
          >
            Filter
          </button>
        {/if}
        {#each VIEWS as [name, word] (name)}
          <button type="button" aria-pressed={view === name} onclick={() => (view = name)}>
            {word}
          </button>
        {/each}
      </div>
    </div>

    {#if kinds.length + states.length > 0}
      <fieldset class="flex flex-wrap gap-x-4 gap-y-1 px-5 pb-2 text-sm" hidden={!filtersOpen}>
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
      <table class="org-calendar-grid" aria-label={monthName(year, month)}>
        <thead>
          <tr>
            {#each weekDays(options.firstDay) as name (name)}
              <th scope="col">{name}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each weeks as week (week[0]?.day)}
            <tr>
              {#each week as day (day.day)}
                {@const events = byDay.get(day.day) ?? []}
                <td
                  class="h-24 p-1"
                  class:org-today={day.day === now}
                  aria-current={day.day === now ? 'date' : undefined}
                >
                  <div class="text-right text-xs" class:opacity-40={!day.inMonth}>
                    {day.number}
                  </div>
                  {#each events.slice(0, options.limit) as event (event.id)}
                    <button
                      type="button"
                      class="block w-full cursor-pointer truncate text-left text-[13px]"
                      style:color={stateColour(event)}
                      title="{event.title}: {stateOf(event).word}"
                      translate="no"
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
    {:else if view === 'week' || view === 'day'}
      <table class="org-calendar-grid" aria-label={title}>
        <thead>
          <tr>
            {#each columns as day (day)}
              <th scope="col" class:org-today={day === now}>{shortDay(day)}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          <tr>
            {#each columns as day (day)}
              <td
                class="h-40"
                class:org-today={day === now}
                aria-current={day === now ? 'date' : undefined}
              >
                {#each byDay.get(day) ?? [] as event (event.id)}
                  {@const icon = KINDS[event.imagetypeFilter]?.icon.split('::')[1] ?? 'tv'}
                  <button
                    type="button"
                    class="org-calendar-event"
                    title={stateOf(event).word}
                    onclick={() => (open = event)}
                  >
                    <span class="font-medium" style:color={stateColour(event)}>
                      <i class="oi oi-fontawesome fa-{icon}" aria-hidden="true"></i>
                      {shortTime(event.start) ?? ''}
                    </span>
                    <span class="block truncate">{event.details.topTitle}</span>
                    <span class="block truncate text-[11px]">{event.details.bottomTitle}</span>
                    <span class="sr-only">{stateOf(event).word}</span>
                  </button>
                {/each}
              </td>
            {/each}
          </tr>
        </tbody>
      </table>
    {:else if listDays.length === 0}
      <p class="px-5 pb-3 text-sm">The calendar has nothing for this month.</p>
    {:else}
      <div class="flex max-h-[600px] flex-col gap-2 overflow-y-auto px-5 pb-3">
        {#each listDays as day (day)}
          <section aria-label={longDay(day)}>
            <h3 class="text-sm font-medium opacity-70">{longDay(day)}</h3>
            <ul>
              {#each byDay.get(day) ?? [] as event (event.id)}
                <li>
                  <button
                    type="button"
                    class="hover:bg-base-100 flex w-full cursor-pointer items-center gap-2 p-1 text-left"
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
                    <span class="text-xs" style:color={stateColour(event)}>
                      {stateOf(event).word}
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      </div>
    {/if}
  </div>
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
      <span class="badge badge-outline">{longDay(dayOf(event.start))}</span>
      {#each event.releaseType ?? [] as release (release)}
        <span class="badge badge-outline">{release}</span>
      {/each}
      {#if event.details.runtime}
        <span class="badge badge-outline">{event.details.runtime} minutes</span>
      {/if}
      {#if event.details.ratings && event.details.ratings !== 'unknown'}
        <span class="badge badge-outline">Rating {event.details.ratings}</span>
      {/if}
      {#each genresOf(event) as genre (genre)}
        <span class="badge badge-ghost">{genre}</span>
      {/each}
    </p>
    {#if event.details.overview}
      <p class="text-sm">{event.details.overview}</p>
    {/if}
    <div class="modal-action">
      {#if event.details.showLink && /^https?:\/\//.test(event.details.href ?? '')}
        <a class="btn" href={event.details.href} target="_blank" rel="noopener noreferrer">
          Open in the application
        </a>
      {/if}
      <button type="button" class="btn btn-primary" onclick={() => (open = null)}>Close</button>
    </div>
  </Dialog>
{/if}
