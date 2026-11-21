<script lang="ts">
  // System Settings, SSO: the form of single sign-on of PHP Organizr, with a list of the
  // Plex servers of the stored token in its group Plex.
  import { api, failureText } from '../../lib/api';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  /** A Plex server, as `GET plex/servers` describes it. */
  interface PlexServer {
    name: string;
    address: string;
    machineIdentifier: string;
    owned: number;
  }

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let servers = $state<PlexServer[] | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function listServers() {
    busy = true;
    error = null;
    try {
      servers = (await api<PlexServer[]>('GET', 'plex/servers')).data;
    } catch (failure) {
      error = failureText(failure);
      servers = null;
    } finally {
      busy = false;
    }
  }
</script>

{#snippet plexServers()}
  <section class="mb-6" aria-labelledby="plex-servers-title">
    <h2 id="plex-servers-title" class="text-base font-semibold">Find the id of your Plex server</h2>
    <p class="my-2 text-sm">
      Save the Plex token first. The list shows the servers of that Plex account. Copy the id of
      your server into the field above.
    </p>
    {#if error}
      <div role="alert" class="alert alert-error mb-4">
        <span>{error}</span>
      </div>
    {/if}
    <button type="button" class="btn" disabled={busy} onclick={listServers}>
      {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
      List the Plex servers
    </button>
    {#if servers}
      {#if servers.length === 0}
        <p class="mt-4 text-sm">The Plex account has no server.</p>
      {:else}
        <div class="mt-4 overflow-x-auto">
          <table class="table">
            <caption class="sr-only">The Plex servers of the stored token</caption>
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Id</th>
                <th scope="col">Owner</th>
              </tr>
            </thead>
            <tbody>
              {#each servers as server (server.machineIdentifier)}
                <tr>
                  <th scope="row">{server.name}</th>
                  <td><code class="break-all">{server.machineIdentifier}</code></td>
                  <td>{server.owned === 1 ? 'This account' : 'Another account'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  </section>
{/snippet}

<PhpSettingsForm
  path="settings/sso"
  title="Single Sign-On"
  extra={{ Plex: plexServers }}
  {ondone}
/>
