<script lang="ts">
  // The part of the account page that links the account to a provider of the OpenID
  // Connect login. After the link, the person can log in through that provider, also with
  // an Admin account, which a login never links by itself.
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { linkAddress, linkedNotice, providerList, type OidcProvider } from '../lib/oidc';
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
  <section class="card bg-base-200 mt-4" aria-labelledby="oidc-links-title">
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
        <div class="card-actions">
          {#each providers.filter((provider) => provider.key !== linked?.provider) as provider (provider.key)}
            <a class="btn btn-secondary" href={linkAddress(provider.key)}>
              Link to {provider.name}
            </a>
          {/each}
        </div>
      {/if}
    </div>
  </section>
{/if}
