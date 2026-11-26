<script lang="ts">
  // The part of the account page that links the account to a provider of the OpenID
  // Connect login. After the link, the person can log in through that provider, also with
  // an Admin account, which a login never links by itself.
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { browser, linkedNotice, linkPath, providerList, type OidcProvider } from '../lib/oidc';
  import { session } from '../lib/session.svelte';

  /** What `GET oidc/link` answers. */
  interface Linked {
    provider: string | null;
    name: string | null;
  }

  const enabled = $derived(session.launch?.settings.login?.oidc?.enabled === true);
  let providers = $state<OidcProvider[]>([]);
  let linked = $state<Linked | null>(null);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let password = $state('');
  let busy = $state(false);

  /** The providers that the account can be linked to: those it is not linked to. */
  const others = $derived(providers.filter((provider) => provider.key !== linked?.provider));

  /** Unlinks the account, with its password, so that it logs in with the password again. */
  async function unlink(event: SubmitEvent) {
    event.preventDefault();
    if (!linked?.provider) {
      return;
    }
    busy = true;
    try {
      const answer = await api('DELETE', 'oidc/link', { password });
      password = '';
      await load();
      done = answer.message ?? 'Your account is no longer linked.';
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  /**
   * Starts the link to a provider, with the password of the account: the server answers
   * with the address of the login page of the provider, and the browser goes there.
   */
  async function link(provider: OidcProvider) {
    done = null;
    if (password === '') {
      error = 'Enter the password of this account first.';
      return;
    }
    busy = true;
    try {
      const answer = await api<{ address: string }>('POST', linkPath(provider.key), { password });
      browser.go(answer.data.address);
    } catch (failure) {
      error = failureText(failure);
      busy = false;
    }
  }

  async function load() {
    try {
      const [list, link] = await Promise.all([
        api('GET', 'oidc/providers'),
        api<Linked>('GET', 'oidc/link'),
      ]);
      providers = providerList(list.data);
      linked = link.data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  onMount(() => {
    if (!enabled) {
      return;
    }
    // The server sends the browser back here after a link, with the provider in the query.
    const key = linkedNotice(window.location.search);
    if (key !== null) {
      history.replaceState(null, '', `${window.location.pathname}${window.location.hash}`);
    }
    void load().then(() => {
      if (key !== null && linked?.provider === key) {
        done = `Your account is linked to ${linked.name ?? key}.`;
      }
    });
  });
</script>

{#if enabled}
  <section class="card org-account-panel" aria-labelledby="oidc-links-title">
    <div class="card-body">
      <h2 id="oidc-links-title" class="card-title">Log in through a provider</h2>
      <p class="text-sm">
        Link your account to a provider of the login. You then log in there, and need no password
        here.
      </p>
      <p role="status" class="text-success min-h-6 text-sm">{done ?? ''}</p>
      {#if error}
        <div role="alert" class="alert alert-error">
          <span>{error}</span>
        </div>
      {/if}
      {#if linked}
        <p>
          {#if linked.provider}
            Your account is linked to {linked.name ?? linked.provider}.
          {:else}
            Your account is not linked to a provider.
          {/if}
        </p>
        {#if linked.provider || others.length > 0}
          <!-- A link and an unlink both need the password of the account. -->
          <form class="flex flex-wrap items-end gap-2" onsubmit={unlink}>
            <label class="form-control">
              <span class="label-text">Password of this account</span>
              <input
                class="input input-bordered input-sm"
                type="password"
                autocomplete="current-password"
                required
                bind:value={password}
              />
            </label>
            {#if linked.provider}
              <button class="btn btn-outline btn-error btn-sm" type="submit" disabled={busy}>
                Unlink from {linked.name ?? linked.provider}
              </button>
            {/if}
          </form>
          <p class="text-xs opacity-70">
            A link and an unlink need this password. After an unlink you log in with it. An account
            that the provider made has a random password: reset it by e-mail first.
          </p>
        {/if}
        <div class="card-actions">
          {#each others as provider (provider.key)}
            <button
              class="btn btn-secondary"
              type="button"
              disabled={busy}
              onclick={() => link(provider)}
            >
              Link to {provider.name}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </section>
{/if}
