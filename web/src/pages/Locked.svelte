<script lang="ts">
  // The lock screen: the account is logged in, and the pages stay closed until the
  // person enters the password.
  import { api, failureText } from '../lib/api';
  import { afterLogout, browser } from '../lib/oidc';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  let password = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** The picture behind the lock screen, as PHP takes it: that of the login page. */
  const wallpaper = $derived.by(() => {
    const appearance = session.launch?.appearance;
    if (appearance?.randomMediaImage) {
      return appearance.randomMediaImage;
    }
    const first = (appearance?.loginWallpaper ?? '').split(',')[0]?.trim();
    return first || 'plugins/images/login-register.jpg';
  });

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
      const oidc = session.launch?.settings.login?.oidc;
      await api('GET', 'logout');
      const away = afterLogout(oidc);
      if (away !== null) {
        browser.go(away);
        return;
      }
      router.go(PAGES.home);
    });
  }
</script>

<main class="org-lock-screen" style:background-image={`url('${wallpaper}')`}>
  <form class="org-lock-box" onsubmit={unlock}>
    <h1 class="sr-only">The screen is locked</h1>
    <div class="text-center">
      <img
        class="mx-auto size-[100px] rounded-full object-cover"
        alt=""
        src={session.launch?.user?.image || 'plugins/images/default_user.png'}
      />
      <h2 class="org-lock-name mt-3 mb-6 text-2xl font-normal text-white">
        {session.launch?.user?.username}
      </h2>
    </div>
    <label class="sr-only" for="unlock-password">Password</label>
    <input
      id="unlock-password"
      class="input w-full"
      type="password"
      placeholder="password"
      bind:value={password}
      required
      autocomplete="current-password"
    />
    {#if error}
      <div role="alert" class="alert alert-error mt-4"><span>{error}</span></div>
    {/if}
    <button class="btn btn-primary org-login-button mt-6 w-full" disabled={busy}>
      {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
      Unlock
    </button>
    <button
      type="button"
      class="link link-hover mt-4 block w-full text-center text-sm"
      disabled={busy}
      onclick={logOut}
    >
      Logout
    </button>
  </form>
</main>
