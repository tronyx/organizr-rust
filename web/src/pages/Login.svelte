<script lang="ts">
  // The login page.
  import { api, ApiError, failureText } from '../lib/api';
  import { takeReturn } from '../lib/returnTo';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  /** What the server answers when the account has a second factor and no code came. */
  const CODE_NEEDED = '2FA Code Needed';

  let username = $state('');
  let password = $state('');
  let remember = $state(false);
  // The code of the second factor. The field shows once the server asks for a code.
  let code = $state('');
  let needsCode = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);

  // The server sends a new password by e-mail only when its mail plugin is on.
  const canRecover = $derived(session.launch?.plugins['PHPMAILER-enabled'] === true);

  async function logIn(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      // The server takes any value of `remember` for "yes", so the field is left out
      // for "no".
      await api('POST', 'login', {
        username,
        password,
        ...(remember ? { remember: true } : {}),
        ...(needsCode ? { tfaCode: code } : {}),
      });
      await session.load();
      // A reverse proxy sent the person here. The login is done, so back to that page.
      const back = takeReturn(window.location.host);
      if (back !== null) {
        window.location.assign(back);
        return;
      }
      router.go(PAGES.home);
    } catch (failure) {
      if (failure instanceof ApiError && failure.message === CODE_NEEDED) {
        needsCode = true;
      } else if (failure instanceof ApiError && failure.message === 'Wrong 2FA') {
        error = 'This code is not right. Enter the code that the app shows now.';
        code = '';
      } else {
        error = failureText(failure);
      }
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
        {#if needsCode}
          <label class="label" for="login-code">Code from your authenticator app</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="login-code"
            class="input w-full"
            bind:value={code}
            required
            autofocus
            inputmode="numeric"
            autocomplete="one-time-code"
            aria-describedby="login-code-help"
          />
          <p id="login-code-help" class="label whitespace-normal">
            Your account has a second factor. Enter the six digits that the app shows now.
          </p>
        {/if}
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
        {#if canRecover}
          <a class="link link-hover w-full text-center text-sm" href={PAGES.recover}>
            Forgot your password?
          </a>
        {/if}
        <a class="link link-hover w-full text-center text-sm" href={PAGES.register}>
          Make an account
        </a>
      </div>
    </div>
  </form>
</main>
