<script lang="ts">
  // The login page.
  import { api, failureText } from '../lib/api';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  let username = $state('');
  let password = $state('');
  let remember = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function logIn(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      // The server takes any value of `remember` for "yes", so the field is left out
      // for "no".
      await api('POST', 'login', { username, password, ...(remember ? { remember: true } : {}) });
      await session.load();
      router.go(PAGES.home);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<main class="grid min-h-[70vh] place-items-center p-4">
  <form class="card bg-base-200 w-full max-w-sm shadow" onsubmit={logIn}>
    <div class="card-body">
      <h1 class="card-title">Log in</h1>
      <fieldset class="fieldset">
        <label class="label" for="login-username">User name or e-mail address</label>
        <input
          id="login-username"
          class="input w-full"
          bind:value={username}
          required
          autocomplete="username"
        />
        <label class="label" for="login-password">Password</label>
        <input
          id="login-password"
          class="input w-full"
          type="password"
          bind:value={password}
          required
          autocomplete="current-password"
        />
        <label class="label mt-2">
          <input type="checkbox" class="checkbox" bind:checked={remember} />
          Stay logged in on this device
        </label>
      </fieldset>

      {#if error}
        <div role="alert" class="alert alert-error">
          <span>{error}</span>
        </div>
      {/if}

      <div class="card-actions">
        <button class="btn btn-primary w-full" disabled={busy}>
          {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
          Log in
        </button>
        <a class="link link-hover w-full text-center text-sm" href={PAGES.register}>
          Make an account
        </a>
      </div>
    </div>
  </form>
</main>
