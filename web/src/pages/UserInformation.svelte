<script lang="ts">
  // The panel "User Information" of the account page of PHP Organizr: the person changes
  // the user name, the e-mail address and the password of the own account, with
  // `PUT /users/<id>`. Only the fields that changed are sent; a new password only when
  // both fields hold the same text.
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { session } from '../lib/session.svelte';

  /** What `GET /token/me` says about the own account. */
  interface Me {
    userID: number;
    username?: string;
    email: string | null;
    authService?: string;
  }

  let me = $state<Me | null>(null);
  let username = $state('');
  let email = $state('');
  let password = $state('');
  let verify = $state('');
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let busy = $state(false);

  /** The fields that differ from the account as stored. */
  const changes = $derived.by(() => {
    const fields: Record<string, string> = {};
    if (me && username.trim() !== (me.username ?? '')) {
      fields.username = username.trim();
    }
    if (me && email.trim() !== (me.email ?? '')) {
      fields.email = email.trim();
    }
    if (password !== '') {
      fields.password = password;
    }
    return fields;
  });

  async function load() {
    try {
      me = (await api<Me>('GET', 'token/me')).data;
      username = me.username ?? '';
      email = me.email ?? '';
      error = null;
    } catch {
      // The account page says so already.
      me = null;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!me) {
      return;
    }
    if (password !== verify) {
      error = 'The two passwords are not the same.';
      done = null;
      return;
    }
    busy = true;
    try {
      await api('PUT', `users/${me.userID}`, changes);
      password = '';
      verify = '';
      await Promise.all([load(), session.load()]);
      error = null;
      done = 'Your account is saved.';
    } catch (failure) {
      error = failureText(failure);
      done = null;
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

<section class="card org-account-panel" aria-labelledby="user-information-title">
  <div class="card-body">
    <h2 id="user-information-title" class="card-title">User Information</h2>
    {#if me?.authService && me.authService !== 'internal'}
      <div class="org-notice org-notice-warning mb-4">
        <h3>Password Notice</h3>
        <p>
          You log in through another service. Change your password there; a password set here is
          only for the login of Organizr itself.
        </p>
      </div>
    {/if}
    {#if done}
      <p role="status" class="text-success text-sm">{done}</p>
    {/if}
    {#if error}
      <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}
    {#if me}
      <form class="org-form-grid" onsubmit={save}>
        <div class="org-form-group">
          <label class="org-form-label" for="account-username">Username</label>
          <input
            id="account-username"
            class="org-form-control"
            bind:value={username}
            required
            autocomplete="username"
          />
        </div>
        <div class="org-form-group">
          <label class="org-form-label" for="account-email">Email</label>
          <input
            id="account-email"
            class="org-form-control"
            type="email"
            bind:value={email}
            autocomplete="email"
          />
        </div>
        <div class="org-form-group">
          <label class="org-form-label" for="account-password">Password</label>
          <input
            id="account-password"
            class="org-form-control"
            type="password"
            bind:value={password}
            autocomplete="new-password"
          />
        </div>
        <div class="org-form-group">
          <label class="org-form-label" for="account-verify">Verify Password</label>
          <input
            id="account-verify"
            class="org-form-control"
            type="password"
            bind:value={verify}
            autocomplete="new-password"
          />
        </div>
        <div class="org-form-wide">
          <button class="btn btn-info btn-sm" disabled={busy || Object.keys(changes).length === 0}>
            {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
            Save
          </button>
        </div>
      </form>
    {/if}
  </div>
</section>
