<script lang="ts">
  // The Overseerr item and the Ombi item: the requests for films and series, and what an
  // Admin can do with one.
  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import { itemData } from '../../lib/itemData.svelte';
  import { pictureAddress, posterAddress, showNoPoster } from '../../lib/plex';
  import PosterRow from './PosterRow.svelte';
  import RequestSearch from './RequestSearch.svelte';
  import { session } from '../../lib/session.svelte';
  import { fromNow } from '../../lib/time';
  import {
    hiddenAtStart,
    REQUEST_SERVICES,
    type RequestOptions,
    type RequestService,
    type RequestState,
    requestState,
  } from '../../lib/requests';

  interface Request {
    id: number | null;
    request_id: number;
    type: 'movie' | 'tv';
    title: string | null;
    overview: string | null;
    poster: string;
    background: string;
    approved: boolean;
    available: boolean;
    denied: boolean;
    user: string;
    userAlias: string | null;
    request_date: string | null;
    release_date: string | null;
  }

  interface Answer {
    count: { movie: number; tv: number };
    content: Request[];
  }

  /** The states of a request, with the word and the color of each one. */
  const STATES = {
    available: { word: 'Available', badge: 'badge-success' },
    approved: { word: 'Approved', badge: 'badge-info' },
    denied: { word: 'Denied', badge: 'badge-error' },
    pending: { word: 'Waiting', badge: 'badge-warning' },
  } as const;
  type State = RequestState;

  let {
    service = 'overseerr',
    options = {},
  }: { service?: RequestService; options?: RequestOptions } = $props();

  // svelte-ignore state_referenced_locally
  const name = REQUEST_SERVICES[service];
  // svelte-ignore state_referenced_locally
  const item = itemData<Answer>(`homepage/${service}/requests`, `${service}Refresh`);

  // svelte-ignore state_referenced_locally
  let hidden = $state<State[]>(hiddenAtStart(options));
  let open = $state<Request | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);

  const stateOf = requestState;

  /** Whether the person may make a request here: the level, and the session of the service. */
  const canRequest = $derived(session.launch?.settings.homepage[service]?.enabled === true);

  /** The name of the person who made a request, as the settings show it. */
  function requesterOf(request: Request): string {
    return (options.alias === false ? '' : request.userAlias) || request.user;
  }

  const all = $derived(item.data?.content ?? []);
  const states = $derived(
    (Object.keys(STATES) as State[]).filter((state) => all.some((one) => stateOf(one) === state)),
  );
  const shown = $derived(all.filter((request) => !hidden.includes(stateOf(request))));

  function toggle(state: State) {
    hidden = hidden.includes(state) ? hidden.filter((one) => one !== state) : [...hidden, state];
  }

  /** Changes a request, and loads the list again. */
  async function change(request: Request, method: string, action: string) {
    busy = true;
    error = null;
    try {
      const path = `homepage/${service}/requests/${request.type}/${request.request_id}${action}`;
      done = (await api(method, path)).message;
      open = null;
      await item.reload();
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>{name}: {item.error}</span></div>
{:else if item.data}
  <p role="status" class="text-success text-sm">{done ?? ''}</p>
  {#snippet filters()}
    {#each states as state (state)}
      <label>
        <input
          type="checkbox"
          class="checkbox checkbox-xs"
          checked={!hidden.includes(state)}
          onchange={() => toggle(state)}
        />
        {STATES[state].word}
      </label>
    {/each}
  {/snippet}
  {#snippet searchButton()}
    <RequestSearch
      {service}
      onrequested={(message) => {
        done = message;
        void item.reload();
      }}
    />
  {/snippet}
  <PosterRow
    title="Requests"
    label="Requests"
    {filters}
    buttons={canRequest ? searchButton : undefined}
  >
    {#each shown as request (request.request_id)}
      {@const picture = posterAddress(request.poster)}
      {@const state = STATES[stateOf(request)]}
      <li class="org-poster">
        <button
          type="button"
          title="{request.title ?? ''}: {state.word}"
          onclick={() => {
            open = request;
            error = null;
          }}
        >
          <img src={picture} alt="" loading="lazy" onerror={showNoPoster} />
          <!-- The color of the corner says how far the request is. -->
          <span
            class="org-poster-corner"
            style:--org-state="var(--color-{state.badge.replace('badge-', '')})"
            aria-hidden="true"
          ></span>
          <span class="sr-only">{request.title ?? ''}</span>
          <span class="sr-only">{state.word}</span>
        </button>
      </li>
    {/each}
  </PosterRow>
  {#if shown.length === 0}
    <p class="text-sm">There are no requests to show.</p>
  {/if}
{/if}

{#if open}
  {@const request = open}
  {@const picture = pictureAddress(request.background)}
  <Dialog title={request.title ?? 'Request'} onclose={() => (open = null)}>
    {#if picture}
      <img class="my-2 max-h-64 w-full rounded object-cover" src={picture} alt="" />
    {/if}
    <p class="my-2 flex flex-wrap gap-1">
      <span class="badge {STATES[stateOf(request)].badge}">{STATES[stateOf(request)].word}</span>
      <span class="badge badge-outline">{request.type === 'movie' ? 'Film' : 'Series'}</span>
      {#if request.release_date}
        <span class="badge badge-outline">{request.release_date.slice(0, 4)}</span>
      {/if}
    </p>
    <p class="text-sm opacity-70">
      Requested by {requesterOf(request)}
      {fromNow(request.request_date) ?? ''}
    </p>
    {#if request.overview}
      <p class="mt-2 text-sm">{request.overview}</p>
    {/if}
    {#if error}
      <div role="alert" class="alert alert-error mt-2"><span>{error}</span></div>
    {/if}
    <div class="modal-action flex-wrap">
      {#if session.canManage}
        {#if !request.approved && !request.available}
          <button
            type="button"
            class="btn btn-sm"
            disabled={busy}
            onclick={() => change(request, 'POST', '/approve')}
          >
            Approve
          </button>
        {/if}
        {#if !request.denied && !request.available}
          <button
            type="button"
            class="btn btn-sm"
            disabled={busy}
            onclick={() => change(request, 'PUT', '/deny')}
          >
            Deny
          </button>
        {/if}
        <button
          type="button"
          class="btn btn-sm"
          disabled={busy}
          onclick={() => change(request, 'POST', request.available ? '/unavailable' : '/available')}
        >
          {request.available ? 'Mark as not available' : 'Mark as available'}
        </button>
        <button
          type="button"
          class="btn btn-sm btn-error"
          disabled={busy}
          onclick={() => change(request, 'DELETE', '')}
        >
          Delete
        </button>
      {/if}
      <button type="button" class="btn btn-sm btn-primary" onclick={() => (open = null)}>
        Close
      </button>
    </div>
  </Dialog>
{/if}
