<script lang="ts">
  // The page where a person who knows the registration password makes an account.
  import LoginLayout from '../components/LoginLayout.svelte';
  import { api, failureText } from '../lib/api';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  let username = $state('');
  let email = $state('');
  let password = $state('');
  let passwordAgain = $state('');
  let registrationPassword = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  const passwordsDiffer = $derived(passwordAgain !== '' && password !== passwordAgain);

  async function register(event: SubmitEvent) {
    event.preventDefault();
    if (password !== passwordAgain) {
      error = 'The two passwords are not the same.';
      return;
    }
    busy = true;
    error = null;
    try {
      await api('POST', 'users/register', { username, email, password, registrationPassword });
      // The server logs the new user in.
      await session.load();
      router.go(PAGES.home);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<LoginLayout>
  <form class="org-login-form card bg-base-200 w-full max-w-sm shadow" onsubmit={register}>
    <div class="card-body">
      <h1 class="card-title">Make an account</h1>
      <fieldset class="fieldset">
        <label class="label" for="register-username">User name</label>
        <input
          id="register-username"
          class="input w-full"
          bind:value={username}
          required
          maxlength="50"
          autocomplete="username"
        />
        <label class="label" for="register-email">E-mail address</label>
        <input
          id="register-email"
          class="input w-full"
          type="email"
          bind:value={email}
          required
          autocomplete="email"
        />
        <label class="label" for="register-password">Password</label>
        <input
          id="register-password"
          class="input w-full"
          type="password"
          bind:value={password}
          required
          autocomplete="new-password"
        />
        <label class="label" for="register-password-again">Password, once more</label>
        <input
          id="register-password-again"
          class="input w-full"
          class:input-error={passwordsDiffer}
          type="password"
          bind:value={passwordAgain}
          required
          autocomplete="new-password"
          aria-invalid={passwordsDiffer}
        />
        <label class="label" for="register-registration-password">Registration password</label>
        <input
          id="register-registration-password"
          class="input w-full"
          type="password"
          bind:value={registrationPassword}
          required
          autocomplete="off"
          aria-describedby="register-registration-password-hint"
        />
        <p id="register-registration-password-hint" class="label whitespace-normal">
          The owner of this install gives this password to the people who may use it.
        </p>
      </fieldset>

      {#if error}
        <div role="alert" class="alert alert-error">
          <span>{error}</span>
        </div>
      {/if}

      <div class="card-actions">
        <button class="btn btn-primary w-full" disabled={busy}>
          {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
          Make the account
        </button>
        <a class="link link-hover w-full text-center text-sm" href={PAGES.login}>
          I have an account
        </a>
      </div>
    </div>
  </form>
</LoginLayout>
