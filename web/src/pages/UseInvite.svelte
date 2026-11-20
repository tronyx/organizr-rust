<script lang="ts">
  // The page for a person with an invite code: the code and the name of the Plex account
  // give that account access to the Plex server.
  import { api, failureText } from '../lib/api';
  import { PAGES } from '../lib/route.svelte';

  let {
    code: given = '',
  }: {
    /** The code from the address of the page. */
    code?: string;
  } = $props();

  // The person can change the code that the address gave.
  // svelte-ignore state_referenced_locally
  let code = $state(given);
  let account = $state('');
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let busy = $state(false);

  async function use(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      const path = `plugins/invites/${encodeURIComponent(code.trim())}`;
      const answer = await api('POST', path, { usedby: account });
      done = answer.message ?? 'The code is used.';
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<main class="grid min-h-[70vh] place-items-center p-4">
  <form class="card bg-base-200 w-full max-w-sm shadow" onsubmit={use}>
    <div class="card-body">
      <h1 class="card-title">Use an invite code</h1>
      {#if done}
        <div role="status" class="alert alert-success">
          <span>
            {done}. Plex sent an invitation to the account. Accept the invitation in Plex, or in the
            e-mail from Plex.
          </span>
        </div>
      {:else}
        <p class="text-sm">
          Enter your invite code and the user name or the e-mail address of your Plex account.
        </p>
        <fieldset class="fieldset">
          <label class="label" for="invite-code">Invite code</label>
          <input
            id="invite-code"
            class="input w-full font-mono"
            bind:value={code}
            required
            maxlength="50"
            autocomplete="off"
          />

          <label class="label" for="invite-account">Plex user name or e-mail address</label>
          <input
            id="invite-account"
            class="input w-full"
            bind:value={account}
            required
            maxlength="255"
          />
        </fieldset>

        {#if error}
          <div role="alert" class="alert alert-error">
            <span>{error}</span>
          </div>
        {/if}
      {/if}

      <div class="card-actions">
        {#if !done}
          <button class="btn btn-primary w-full" disabled={busy}>
            {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
            Use the code
          </button>
        {/if}
        <a class="link link-hover w-full text-center text-sm" href={PAGES.home}>
          Back to the start
        </a>
      </div>
    </div>
  </form>
</main>
