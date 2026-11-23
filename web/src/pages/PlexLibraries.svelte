<script lang="ts">
  // The page of the Plex Libraries plugin (D-121): the shares of the Plex server that the
  // person sees, each with a switch for each library. A switch that the server does not
  // let the person change is shown, but cannot be used.
  import { onMount } from 'svelte';

  import Notice from '../components/Notice.svelte';
  import { api, failureText } from '../lib/api';

  interface Library {
    id: string;
    title: string;
    kind: string;
    shared: boolean;
    changeable: boolean;
  }

  interface Share {
    id: string;
    username: string;
    email: string;
    libraries: Library[];
  }

  interface Answer {
    manager: boolean;
    shares: Share[];
  }

  let { title }: { title: string } = $props();

  let answer = $state<Answer | null>(null);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let busy = $state<string | null>(null);

  async function load() {
    try {
      answer = (await api<Answer>('GET', 'plugins/plexlibraries/shares')).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Switches one library of a share, and sends the libraries that the share holds then. */
  async function toggle(share: Share, library: Library) {
    busy = share.id;
    done = null;
    const libraries = share.libraries
      .filter((one) => (one.id === library.id ? !one.shared : one.shared))
      .map((one) => one.id);
    try {
      const sent = await api('PUT', `plugins/plexlibraries/shares/${share.id}`, { libraries });
      library.shared = !library.shared;
      done = sent.message ?? 'The libraries are changed';
      error = null;
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = null;
    }
  }

  onMount(load);
</script>

{#if error && answer === null}
  <Notice {title}>
    <div role="alert" class="alert alert-error"><span>{error}</span></div>
  </Notice>
{:else if answer === null}
  <main class="grid min-h-[60vh] place-items-center" aria-busy="true">
    <span class="loading loading-spinner loading-lg" aria-label="The shares are loading"></span>
  </main>
{:else if answer.shares.length === 0}
  <Notice {title}>
    <p>
      {answer.manager
        ? 'The Plex server is shared with nobody yet.'
        : 'Your account has no share of the Plex server.'}
    </p>
  </Notice>
{:else}
  <main class="flex flex-col gap-6 p-4" aria-label={title}>
    {#if done}
      <div role="status" class="alert alert-success"><span>{done}</span></div>
    {/if}
    {#if error}
      <div role="alert" class="alert alert-error"><span>{error}</span></div>
    {/if}
    {#each answer.shares as share (share.id)}
      <section aria-label={share.username} class="org-panel">
        <div class="org-panel-heading">
          <h2 translate="no">{share.username}</h2>
          {#if answer.manager && share.email}
            <span class="text-sm opacity-70" translate="no">{share.email}</span>
          {/if}
        </div>
        <ul class="org-panel-body grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {#each share.libraries as library (library.id)}
            <li>
              <label class="flex cursor-pointer items-center gap-3">
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  checked={library.shared}
                  disabled={!library.changeable || busy === share.id}
                  onchange={() => toggle(share, library)}
                />
                <span translate="no">{library.title}</span>
              </label>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </main>
{/if}
