<script lang="ts">
  // The panel "Active Tokens" of the account page of PHP Organizr: the sessions of the
  // own account, with the button that ends one. The server never sends a token itself,
  // so the column Token says which session is this one.
  import { longDateTime } from '../lib/time';
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';

  /** A session of the own account, as `GET /token/me` lists it. */
  interface Session {
    id: number;
    ip: string | null;
    created: string;
    expires: string;
    current: boolean;
  }

  let sessions = $state<Session[] | null>(null);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);

  /** A time of the server, in UTC, in the local time of this browser, as PHP shows it. */
  function when(text: string): string {
    return longDateTime(text);
  }

  async function load() {
    try {
      const me = (await api<{ tokenList?: Session[] }>('GET', 'token/me')).data;
      sessions = me.tokenList ?? null;
    } catch {
      // The account page says so already.
      sessions = null;
    }
  }

  async function revoke(session: Session) {
    const ask = session.current
      ? 'End this session? You are then logged out.'
      : `End the session ${session.id}?`;
    if (!confirm(ask)) {
      return;
    }
    try {
      await api('DELETE', `token/${session.id}`);
      if (session.current) {
        window.location.reload();
        return;
      }
      await load();
      error = null;
      done = `The session ${session.id} is ended.`;
    } catch (failure) {
      error = failureText(failure);
      done = null;
    }
  }

  onMount(load);
</script>

{#if sessions}
  <section class="card org-account-panel" aria-labelledby="active-tokens-title">
    <div class="card-body">
      <h2 id="active-tokens-title" class="card-title">Active Tokens</h2>
      {#if done}
        <p role="status" class="text-success text-sm">{done}</p>
      {/if}
      {#if error}
        <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
      {/if}
      <div class="overflow-x-auto">
        <table class="org-table">
          <thead>
            <tr>
              <th scope="col">#</th>
              <th scope="col">Token</th>
              <th scope="col">Created</th>
              <th scope="col">Expires</th>
              <th scope="col">IP</th>
              <th scope="col">Action</th>
            </tr>
          </thead>
          <tbody>
            {#each sessions as session (session.id)}
              <tr class:org-current-token={session.current}>
                <th scope="row">{session.id}</th>
                <td>{session.current ? 'This session' : '…'}</td>
                <td>{when(session.created)}</td>
                <td>{when(session.expires)}</td>
                <td>{session.ip ?? ''}</td>
                <td>
                  <button
                    type="button"
                    class="btn btn-error btn-sm"
                    aria-label={`End the session ${session.id}`}
                    onclick={() => revoke(session)}
                  >
                    Revoke
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  </section>
{/if}
