<script lang="ts">
  // The lock screen: the account is logged in, and the pages stay closed until the
  // person enters the password.
  import { api, failureText } from '../lib/api';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  let password = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function act(change: () => Promise<unknown>) {
    busy = true;
    error = null;
    try {
      await change();
      await session.load();
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  function unlock(event: SubmitEvent) {
    event.preventDefault();
    void act(() => api('POST', 'users/unlock', { password }));
  }

  function logOut() {
    void act(async () => {
      await api('GET', 'logout');
      router.go(PAGES.home);
    });
  }
</script>

<main class="grid min-h-screen place-items-center p-4">
  <form class="card bg-base-200 w-full max-w-sm shadow" onsubmit={unlock}>
    <div class="card-body">
      <h1 class="card-title">The screen is locked</h1>
      <p class="text-sm">
        {session.launch?.user?.username}, enter your password to open Organizr again.
      </p>
      <fieldset class="fieldset">
        <label class="label" for="unlock-password">Password</label>
        <input
          id="unlock-password"
          class="input w-full"
          type="password"
          bind:value={password}
          required
          autocomplete="current-password"
        />
      </fieldset>
      {#if error}
        <div role="alert" class="alert alert-error"><span>{error}</span></div>
      {/if}
      <div class="card-actions">
        <button class="btn btn-primary w-full" disabled={busy}>
          {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
          Unlock
        </button>
        <button type="button" class="btn btn-ghost w-full" disabled={busy} onclick={logOut}>
          Log out
        </button>
      </div>
    </div>
  </form>
</main>
