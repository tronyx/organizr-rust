<script lang="ts">
  // The pop-up of PHP Organizr for a new request: "Request a Show or Movie". The server
  // asks the request service for the films and series that match the text (D-108); a
  // click on one asks, and then makes the request as the person.
  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import { REQUEST_SERVICES, type RequestService } from '../../lib/requests';

  /** One match, as `GET homepage/{service}/search/{query}` gives it. */
  interface Found {
    id: number;
    type: 'movie' | 'tv';
    title: string;
    year: string;
    poster: string;
    overview: string;
    status: 'available' | 'requested' | null;
  }

  interface Answer {
    page: number;
    totalPages: number;
    results: Found[];
  }

  let {
    service,
    onrequested,
  }: {
    service: RequestService;
    /** Called after a request was made, with the message of the server. */
    onrequested: (message: string) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  const name = REQUEST_SERVICES[service];

  let open = $state(false);
  let query = $state('');
  let searched = $state('');
  let answer = $state<Answer | null>(null);
  let chosen = $state<Found | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function search(page = 1) {
    const asked = query.trim();
    if (asked === '') {
      return;
    }
    busy = true;
    error = null;
    try {
      const path =
        page === 1
          ? `homepage/${service}/search/${encodeURIComponent(asked)}`
          : `homepage/${service}/search/${encodeURIComponent(asked)}/${page}`;
      answer = (await api<Answer>('GET', path)).data;
      searched = asked;
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  async function request(found: Found) {
    busy = true;
    error = null;
    try {
      const message = (await api('POST', `homepage/${service}/requests/${found.type}/${found.id}`))
        .message;
      chosen = null;
      open = false;
      onrequested(message ?? `${found.title} is requested.`);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  const STATUS_WORDS = { available: 'Available', requested: 'Requested' } as const;
</script>

<button type="button" aria-label="Request a Show or Movie" onclick={() => (open = true)}>
  <i class="oi oi-fontawesome fa-search" aria-hidden="true"></i>
</button>

{#if open}
  <Dialog title="Request a Show or Movie" onclose={() => (open = false)}>
    <form
      class="flex gap-2"
      onsubmit={(event) => {
        event.preventDefault();
        void search();
      }}
    >
      <input
        class="org-form-control"
        type="search"
        placeholder="Request a Show or Movie"
        aria-label="Request a Show or Movie"
        bind:value={query}
      />
      <button type="submit" class="org-form-button" disabled={busy}>
        {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
        Search
      </button>
    </form>
    {#if error}
      <div role="alert" class="alert alert-error mt-3"><span>{error}</span></div>
    {/if}
    {#if answer}
      {#if answer.results.length === 0}
        <p class="mt-3 text-sm">No results for {searched}</p>
      {:else}
        <ul class="org-search-results" aria-label="Results in {name}">
          {#each answer.results as found (`${found.type}-${found.id}`)}
            <li>
              <button type="button" onclick={() => (chosen = found)}>
                <img src={found.poster} alt="" loading="lazy" />
                <span class="truncate text-xs">{found.title} {found.year}</span>
                {#if found.status}
                  <span
                    class="badge badge-sm {found.status === 'available'
                      ? 'badge-success'
                      : 'badge-info'}"
                  >
                    {STATUS_WORDS[found.status]}
                  </span>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
        {#if answer.totalPages > 1}
          <div class="mt-3 flex items-center justify-center gap-3 text-sm">
            <button
              type="button"
              class="org-form-button"
              disabled={busy || answer.page <= 1}
              onclick={() => search((answer?.page ?? 2) - 1)}
            >
              Previous
            </button>
            <span>Page {answer.page} of {answer.totalPages}</span>
            <button
              type="button"
              class="org-form-button"
              disabled={busy || answer.page >= answer.totalPages}
              onclick={() => search((answer?.page ?? 0) + 1)}
            >
              Next
            </button>
          </div>
        {/if}
      {/if}
    {/if}
  </Dialog>
{/if}

{#if chosen}
  <Dialog title={chosen.title} onclose={() => (chosen = null)}>
    {#if chosen.year}<p class="opacity-70">{chosen.year}</p>{/if}
    {#if chosen.overview}<p class="my-2 text-sm">{chosen.overview}</p>{/if}
    <div class="modal-action">
      {#if chosen.status}
        <span class="mr-auto text-sm">{STATUS_WORDS[chosen.status]}</span>
      {:else}
        <button
          type="button"
          class="btn btn-info"
          disabled={busy}
          onclick={() => chosen && request(chosen)}
        >
          Request
        </button>
      {/if}
      <button type="button" class="btn" onclick={() => (chosen = null)}>Close</button>
    </div>
  </Dialog>
{/if}
