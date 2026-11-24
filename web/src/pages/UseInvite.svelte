<script lang="ts">
  // The page for a person with an invite code. The code and the name of a Plex account
  // give that account access to the Plex server. A person without an account can make
  // a Plex account or an Emby user with the code instead.
  import LoginLayout from '../components/LoginLayout.svelte';
  import { api, failureText } from '../lib/api';
  import { PAGES } from '../lib/route.svelte';

  let {
    code: given = '',
  }: {
    /** The code from the address of the page. */
    code?: string;
  } = $props();

  /** What the person does: use an own Plex account, or make a new account. */
  type Mode = 'use' | 'plex' | 'emby';
  const MODES: { mode: Mode; label: string }[] = [
    { mode: 'use', label: 'I have a Plex account' },
    { mode: 'plex', label: 'Make a Plex account' },
    { mode: 'emby', label: 'Make an Emby user' },
  ];

  // The person can change the code that the address gave.
  // svelte-ignore state_referenced_locally
  let code = $state(given);
  let mode = $state<Mode>('use');
  let account = $state('');
  let username = $state('');
  let email = $state('');
  let password = $state('');
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  // The mode of the request that passed.
  let doneMode = $state<Mode>('use');
  let busy = $state(false);

  function request() {
    const trimmed = code.trim();
    if (mode === 'use') {
      return api('POST', `plugins/invites/${encodeURIComponent(trimmed)}`, { usedby: account });
    }
    return api('POST', `${mode}/register`, { code: trimmed, username, email, password });
  }

  async function use(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      const answer = await request();
      doneMode = mode;
      done = answer.message ?? 'The code is used.';
      password = '';
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<LoginLayout>
  <form class="org-login-form card bg-base-200 w-full max-w-sm shadow" onsubmit={use}>
    <div class="card-body">
      <h1 class="card-title">Use an invite code</h1>
      {#if done}
        <div role="status" class="alert alert-success">
          <span>
            {#if doneMode === 'emby'}
              {done}. Log in to Emby with the new name and password.
            {:else}
              {done}. Plex sent an invitation to the account. Accept the invitation in Plex, or in
              the e-mail from Plex.
            {/if}
          </span>
        </div>
      {:else}
        <p class="text-sm">
          Enter your invite code. Then enter the user name or the e-mail address of your Plex
          account, or make a new account.
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
        </fieldset>

        <fieldset class="fieldset">
          <legend class="fieldset-legend">Account</legend>
          {#each MODES as choice (choice.mode)}
            <label class="label cursor-pointer gap-2">
              <input
                type="radio"
                class="radio radio-sm"
                name="invite-mode"
                value={choice.mode}
                bind:group={mode}
              />
              {choice.label}
            </label>
          {/each}
        </fieldset>

        <fieldset class="fieldset">
          {#if mode === 'use'}
            <label class="label" for="invite-account">Plex user name or e-mail address</label>
            <input
              id="invite-account"
              class="input w-full"
              bind:value={account}
              required
              maxlength="255"
            />
          {:else}
            <label class="label" for="invite-username">User name</label>
            <input
              id="invite-username"
              class="input w-full"
              bind:value={username}
              required
              autocomplete="username"
            />
            <label class="label" for="invite-email">E-mail address</label>
            <input
              id="invite-email"
              type="email"
              class="input w-full"
              bind:value={email}
              required
              autocomplete="email"
            />
            <label class="label" for="invite-password">Password</label>
            <input
              id="invite-password"
              type="password"
              class="input w-full"
              bind:value={password}
              required
              autocomplete="new-password"
            />
          {/if}
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
            {mode === 'use' ? 'Use the code' : 'Make the account'}
          </button>
        {/if}
        <a class="link link-hover w-full text-center text-sm" href={PAGES.home}>
          Back to the start
        </a>
      </div>
    </div>
  </form>
</LoginLayout>
