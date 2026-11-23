<script lang="ts">
  // The page of the own account: the second factor of the login.
  //
  // A second factor is a code from an authenticator app. With it, a login needs the
  // password and the code of that moment.
  import { onMount } from 'svelte';

  import { api, ApiError, failureText } from '../lib/api';
  import { session } from '../lib/session.svelte';
  import LookPicker from '../components/LookPicker.svelte';
  import ActiveTokens from './ActiveTokens.svelte';
  import OidcLinks from './OidcLinks.svelte';
  import UserInformation from './UserInformation.svelte';

  /** What `POST 2fa/google` answers: a new secret, and its QR code as a picture. */
  interface NewFactor {
    type: string;
    secret: string;
    url: string;
  }

  /** The only kind of second factor: a code that changes every 30 seconds. */
  const KIND = 'google';

  let hasFactor = $state<boolean | null>(null);
  let setup = $state<NewFactor | null>(null);
  let code = $state('');
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      const me = (await api<{ authService: string }>('GET', 'token/me')).data;
      // The server names the kind of the second factor where the login service stands.
      hasFactor = me.authService === KIND;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function run(action: () => Promise<void>) {
    busy = true;
    error = null;
    done = null;
    try {
      await action();
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  const start = () =>
    run(async () => {
      setup = (await api<NewFactor>('POST', `2fa/${KIND}`)).data;
      code = '';
    });

  function confirm(event: SubmitEvent) {
    event.preventDefault();
    const pending = setup;
    if (!pending) {
      return;
    }
    void run(async () => {
      const proof = { secret: pending.secret, code, type: KIND };
      try {
        await api('POST', '2fa', proof);
      } catch (failure) {
        // The server answers a wrong code with the status 401.
        if (failure instanceof ApiError && failure.status === 401) {
          throw new Error('This code is not right. Enter the code that the app shows now.', {
            cause: failure,
          });
        }
        throw failure;
      }
      await api('PUT', '2fa', { secret: pending.secret, type: KIND });
      setup = null;
      code = '';
      done = 'The second factor is on. Your next login asks for a code.';
      await load();
    });
  }

  const remove = () =>
    run(async () => {
      if (!window.confirm('Switch the second factor off? A login then needs only the password.')) {
        return;
      }
      await api('DELETE', '2fa');
      done = 'The second factor is off.';
      await load();
    });

  onMount(load);
</script>

<main class="org-settings">
  <div class="org-page-title">
    <h1>Account</h1>
    <ol class="org-breadcrumb" aria-label="Where you are">
      <li>Account Information</li>
      <li>{session.launch?.user?.username}</li>
    </ol>
  </div>

  <UserInformation />

  <ActiveTokens />

  <section class="card org-account-panel" aria-labelledby="look-title">
    <div class="card-body">
      <h2 id="look-title" class="card-title">Look</h2>
      <p class="text-sm">Choose the layout and theme that you see. Other people keep their own.</p>
      <LookPicker />
    </div>
  </section>

  <section class="card org-account-panel" aria-labelledby="second-factor-title">
    <div class="card-body">
      <h2 id="second-factor-title" class="card-title">Second factor</h2>
      <p class="text-sm">
        With a second factor, a login needs your password and a code from an authenticator app on
        your phone. The code changes every 30 seconds.
      </p>

      <p role="status" class="text-success min-h-6 text-sm">{done ?? ''}</p>
      {#if error}
        <div role="alert" class="alert alert-error">
          <span>{error}</span>
        </div>
      {/if}

      {#if setup}
        <form onsubmit={confirm}>
          <ol class="list-decimal space-y-3 pl-5 text-sm">
            <li>
              Scan this picture with the authenticator app.
              <img
                class="mt-2 rounded bg-white p-2"
                src={setup.url}
                alt="QR code for the authenticator app"
                width="200"
                height="200"
              />
              If you cannot scan it, enter this key in the app:
              <code class="bg-base-300 rounded px-1 break-all">{setup.secret}</code>
            </li>
            <li>
              <label for="second-factor-code">Enter the code that the app shows.</label>
              <input
                id="second-factor-code"
                class="input mt-2 w-full max-w-40"
                bind:value={code}
                required
                inputmode="numeric"
                autocomplete="one-time-code"
                pattern="\s*[0-9]{'{6}'}\s*"
                title="Six digits"
              />
            </li>
          </ol>
          <div class="card-actions mt-4">
            <button class="btn btn-primary" disabled={busy}>
              {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
              Switch the second factor on
            </button>
            <button type="button" class="btn" disabled={busy} onclick={() => (setup = null)}>
              Cancel
            </button>
          </div>
        </form>
      {:else if hasFactor === true}
        <p>Your account has a second factor.</p>
        <div class="card-actions">
          <button class="btn btn-error btn-outline" disabled={busy} onclick={remove}>
            Switch the second factor off
          </button>
        </div>
      {:else if hasFactor === false}
        <p>Your account has no second factor.</p>
        <div class="card-actions">
          <button class="btn btn-primary" disabled={busy} onclick={start}>
            {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
            Set a second factor up
          </button>
        </div>
      {:else if !error}
        <span class="loading loading-spinner" aria-label="Loading"></span>
      {/if}
    </div>
  </section>

  <OidcLinks />
</main>
