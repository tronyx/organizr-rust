<script lang="ts">
  // The set-up wizard: what a new install shows first. It makes the first Admin account
  // and stores the settings that an install needs.
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { randomKey } from '../lib/random';
  import { session } from '../lib/session.svelte';
  import type { Status } from '../lib/types';

  const DATABASE_NAMES = { sqlite: 'SQLite', postgresql: 'PostgreSQL' } as const;

  let setupCode = $state('');
  let license = $state('personal');
  let username = $state('');
  let email = $state('');
  let password = $state('');
  let passwordAgain = $state('');
  let registrationPassword = $state('');
  let apiKey = $state(randomKey());
  let database = $state<Status['database'] | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const passwordsDiffer = $derived(passwordAgain !== '' && password !== passwordAgain);

  onMount(async () => {
    // The wizard cannot choose the database. It shows which one the server uses.
    try {
      database = (await api<Status>('GET', 'status')).data.database;
    } catch {
      database = null;
    }
  });

  async function setUp(event: SubmitEvent) {
    event.preventDefault();
    if (password !== passwordAgain) {
      error = 'The two passwords are not the same.';
      return;
    }
    busy = true;
    error = null;
    try {
      await api('POST', 'wizard', {
        setupCode: setupCode.trim(),
        license,
        username,
        email,
        password,
        api: apiKey,
        registrationPassword,
      });
      // The server logs the new Admin in. The next answer shows the menu.
      await session.load();
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<main class="org-settings min-h-screen">
  <div class="org-page-title">
    <h1>Organizr Setup Wizard</h1>
    <ol class="org-breadcrumb" aria-label="Where you are">
      <li>Install Type</li>
      <li>Admin Info</li>
      <li>Security</li>
      <li>Verify</li>
    </ol>
  </div>
  <p class="mt-2 opacity-80">
    This install is new. Fill in this page once to make the first Admin account.
  </p>

  <form class="mx-auto max-w-3xl space-y-6" onsubmit={setUp}>
    <fieldset class="fieldset org-wizard-panel">
      <legend class="org-wizard-heading">Proof that this is your server</legend>
      <label class="label" for="setup-code">Set-up code</label>
      <input
        id="setup-code"
        class="input w-full font-mono"
        bind:value={setupCode}
        required
        autocomplete="off"
        autocapitalize="characters"
        spellcheck="false"
        placeholder="XXXX-XXXX-XXXX"
        aria-describedby="setup-code-hint"
      />
      <p id="setup-code-hint" class="label whitespace-normal">
        The server wrote this code to its log when it started. Only a person who can read the log
        can set the install up.
      </p>
    </fieldset>

    <fieldset class="fieldset org-wizard-panel">
      <legend class="org-wizard-heading">The Admin account</legend>
      <label class="label" for="username">User name</label>
      <input
        id="username"
        class="input w-full"
        bind:value={username}
        required
        maxlength="50"
        autocomplete="username"
      />
      <label class="label" for="email">E-mail address</label>
      <input
        id="email"
        class="input w-full"
        type="email"
        bind:value={email}
        required
        autocomplete="email"
      />
      <label class="label" for="password">Password</label>
      <input
        id="password"
        class="input w-full"
        type="password"
        bind:value={password}
        required
        autocomplete="new-password"
      />
      <label class="label" for="password-again">Password, once more</label>
      <input
        id="password-again"
        class="input w-full"
        class:input-error={passwordsDiffer}
        type="password"
        bind:value={passwordAgain}
        required
        autocomplete="new-password"
        aria-invalid={passwordsDiffer}
        aria-describedby="password-again-hint"
      />
      <p id="password-again-hint" class="label text-error" class:invisible={!passwordsDiffer}>
        The two passwords are not the same.
      </p>
    </fieldset>

    <fieldset class="fieldset org-wizard-panel">
      <legend class="org-wizard-heading">The install</legend>
      <label class="label" for="license">Use</label>
      <select id="license" class="select w-full" bind:value={license}>
        <option value="personal">Personal</option>
        <option value="business">Business</option>
      </select>

      <label class="label" for="registration-password">Registration password</label>
      <input
        id="registration-password"
        class="input w-full"
        type="password"
        bind:value={registrationPassword}
        required
        autocomplete="off"
        aria-describedby="registration-password-hint"
      />
      <p id="registration-password-hint" class="label whitespace-normal">
        A person who knows this password can make an account. Give it only to people who may use
        this install.
      </p>

      <label class="label" for="api-key">API key</label>
      <div class="join w-full">
        <input
          id="api-key"
          class="input join-item w-full font-mono"
          value={apiKey}
          readonly
          aria-describedby="api-key-hint"
        />
        <button type="button" class="btn join-item" onclick={() => (apiKey = randomKey())}>
          Make another
        </button>
      </div>
      <p id="api-key-hint" class="label whitespace-normal">
        A script that sends this key has the rights of the Admin. You can read the key later in the
        settings.
      </p>

      {#if database}
        <p class="mt-2">
          The server stores its data in <strong>{DATABASE_NAMES[database]}</strong>. The database is
          chosen when the server starts, and the wizard cannot change it.
        </p>
      {/if}
    </fieldset>

    {#if error}
      <div role="alert" class="alert alert-error">
        <span>{error}</span>
      </div>
    {/if}

    <button class="btn btn-primary" disabled={busy}>
      {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
      Set up
    </button>
  </form>
</main>
