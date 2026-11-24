<script lang="ts">
  // The page for a person who forgot the password: the server sends a new password to
  // the e-mail address of the account.
  import LoginLayout from '../components/LoginLayout.svelte';
  import { api, failureText } from '../lib/api';
  import { PAGES } from '../lib/route.svelte';

  let email = $state('');
  let error = $state<string | null>(null);
  let sent = $state(false);
  let busy = $state(false);

  async function recover(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      await api('POST', 'users/recover', { email });
      sent = true;
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<LoginLayout>
  <form class="org-login-form card bg-base-200 w-full max-w-sm shadow" onsubmit={recover}>
    <div class="card-body">
      <h1 class="card-title">Get a new password</h1>
      {#if sent}
        <div role="status" class="alert alert-success">
          <span>
            If an account has this address, a new password is on its way to it. The new password
            works for one day. Your old password works until you log in with the new one.
          </span>
        </div>
      {:else}
        <p class="text-sm">
          Enter the e-mail address of your account. You get an e-mail with a new password.
        </p>
        <fieldset class="fieldset">
          <label class="label" for="recover-email">E-mail address</label>
          <input
            id="recover-email"
            class="input w-full"
            type="email"
            bind:value={email}
            required
            autocomplete="email"
          />
        </fieldset>

        {#if error}
          <div role="alert" class="alert alert-error">
            <span>{error}</span>
          </div>
        {/if}
      {/if}

      <div class="card-actions">
        {#if !sent}
          <button class="btn btn-primary w-full" disabled={busy}>
            {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
            Send the e-mail
          </button>
        {/if}
        <a class="link link-hover w-full text-center text-sm" href={PAGES.login}>
          Back to the login
        </a>
      </div>
    </div>
  </form>
</LoginLayout>
