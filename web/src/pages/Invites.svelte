<script lang="ts">
  // The page of the invites plugin for the members who make invites.
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { randomKey } from '../lib/random';
  import { PAGES } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  interface Invite {
    id: number;
    code: string;
    date: string;
    email: string;
    username: string;
    dateused: string | null;
    usedby: string | null;
    valid: string;
    invitedby: string;
  }

  /** The number of characters of a new code. */
  const CODE_LENGTH = 6;

  let invites = $state<Invite[] | null>(null);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let busy = $state(false);
  let username = $state('');
  let email = $state('');

  /** Whether this person can delete invites. The server decides again for each one. */
  const canDelete = $derived.by(() => {
    const includes = session.launch?.plugins.includes as Record<string, unknown> | undefined;
    return session.canManage || includes?.['INVITES-allow-delete-include'] === true;
  });

  async function load() {
    try {
      invites = (await api<Invite[]>('GET', 'plugins/invites')).data;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Runs a change, shows what the server says about it, and loads the list again. */
  async function run(change: () => Promise<{ message: string | null }>): Promise<boolean> {
    busy = true;
    error = null;
    done = null;
    try {
      done = (await change()).message;
      await load();
      return true;
    } catch (failure) {
      error = failureText(failure);
      return false;
    } finally {
      busy = false;
    }
  }

  async function add(event: SubmitEvent) {
    event.preventDefault();
    const code = randomKey(CODE_LENGTH).toUpperCase();
    if (await run(() => api('POST', 'plugins/invites', { code, username, email }))) {
      username = '';
      email = '';
    }
  }

  function usedText(invite: Invite): string {
    if (invite.valid === 'Yes') {
      return 'Not used';
    }
    const by = invite.usedby ? ` by ${invite.usedby}` : '';
    const at = invite.dateused ? ` on ${invite.dateused} UTC` : '';
    return `Used${by}${at}`;
  }

  onMount(load);
</script>

<main class="org-settings">
  <div class="org-page-title">
    <h1>Invites</h1>
    <ol class="org-breadcrumb" aria-label="Where you are">
      <li>Plugins</li>
      <li>Invites</li>
    </ol>
  </div>
  <p class="mb-4 max-w-2xl text-sm">
    An invite gives a person access to the Plex server. Give the code to the person. The person
    opens <a class="link" href={PAGES.useInvite}>the page for an invite code</a> and enters the code with
    the name of the own Plex account.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  {#if done}
    <div role="status" class="alert alert-success mb-4"><span>{done}</span></div>
  {/if}

  {#if invites}
    <form class="mb-6 flex flex-wrap items-end gap-2" onsubmit={add}>
      <div>
        <label class="label" for="invite-name">Name of the person</label>
        <input
          id="invite-name"
          class="input input-sm w-52"
          bind:value={username}
          required
          maxlength="255"
        />
      </div>
      <div>
        <label class="label" for="invite-email">E-mail address</label>
        <input
          id="invite-email"
          class="input input-sm w-64"
          type="email"
          bind:value={email}
          required
          maxlength="255"
        />
      </div>
      <button type="submit" class="btn btn-sm btn-primary" disabled={busy}>Make an invite</button>
    </form>

    {#if invites.length === 0}
      <p>There are no invites yet.</p>
    {:else}
      <div class="overflow-x-auto">
        <table class="table-sm table" aria-label="Invites">
          <thead>
            <tr>
              <th scope="col">Code</th>
              <th scope="col">For</th>
              <th scope="col">Made by</th>
              <th scope="col">Made on (UTC)</th>
              <th scope="col">Use</th>
              <th scope="col"><span class="sr-only">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {#each invites as invite (invite.id)}
              <tr>
                <td class="font-mono">{invite.code}</td>
                <td>{invite.username}<br /><span class="text-xs">{invite.email}</span></td>
                <td>{invite.invitedby}</td>
                <td>{invite.date}</td>
                <td>{usedText(invite)}</td>
                <td>
                  {#if canDelete}
                    <button
                      type="button"
                      class="btn btn-ghost btn-xs"
                      disabled={busy}
                      aria-label="Delete the invite {invite.code}"
                      onclick={() => run(() => api('DELETE', `plugins/invites/${invite.code}`))}
                    >
                      Delete
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</main>
