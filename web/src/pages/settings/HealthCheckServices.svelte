<script lang="ts">
  // The services of the health checks plugin: a list to edit, a button that adds the
  // checks of the Healthchecks account, and a button that runs the checks now.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import {
    emptyRow,
    fromSetting,
    importChecks,
    type ServiceRow,
    toSetting,
  } from '../../lib/healthCheckServices';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  /** The results of a run: for each service, what its addresses answered. */
  type Results = Record<string, { 'Service Name'?: string; results?: Record<string, string> }>;

  let rows = $state<ServiceRow[] | null>(null);
  let saved = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);
  let results = $state<Results | null>(null);

  const hasChanges = $derived(rows !== null && JSON.stringify(rows) !== saved);

  async function load() {
    try {
      const config = (await api<Record<string, unknown>>('GET', 'config')).data;
      rows = fromSetting(config['HEALTHCHECKS-all-items']);
      saved = JSON.stringify(rows);
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!rows) {
      return;
    }
    busy = true;
    error = null;
    try {
      await api('PUT', 'config', { 'HEALTHCHECKS-all-items': toSetting(rows) });
      await load();
      ondone('The services are saved.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  /** What the homepage item Healthchecks answers. */
  interface Account {
    content?: { checks?: unknown };
  }

  /**
   * Adds the checks of the Healthchecks account that the list does not have, as the
   * button "Import Services" of PHP. The account is the one of the homepage item
   * Healthchecks. The new services are not saved until the Admin saves the list.
   */
  async function importNow() {
    if (!rows) {
      return;
    }
    busy = true;
    error = null;
    try {
      const account = (await api<Account | null>('GET', 'homepage/healthchecks')).data;
      const checks = account?.content?.checks;
      if (!Array.isArray(checks) || checks.length === 0) {
        error = 'Healthchecks gave no checks.';
        return;
      }
      const imported = importChecks(rows, checks);
      if (imported.readOnly) {
        error = 'The API key of Healthchecks can only read. Use a key with full access.';
        return;
      }
      rows = imported.rows;
      ondone(
        imported.added.length === 0
          ? 'Every check of Healthchecks is in the list already.'
          : `Added ${imported.added.length} ${imported.added.length === 1 ? 'check' : 'checks'}: ${imported.added.join(', ')}. Save the services to keep them.`,
      );
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  async function runNow() {
    busy = true;
    error = null;
    results = null;
    try {
      results = (await api<Results>('GET', 'plugins/healthchecks/run')).data;
      ondone('The checks ran.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

<section class="mt-8" aria-labelledby="health-services-title">
  <h2 id="health-services-title" class="text-base font-semibold">Services of the health checks</h2>
  <p class="my-2 max-w-2xl text-sm">
    Each service has a check at Healthchecks. Enter the id of that check, and one address or both.
    With both, the address from the internet is asked only when the address inside the network does
    not answer. The button that imports the checks adds each check of the Healthchecks account of
    the homepage item Healthchecks that the list does not have yet.
  </p>
  <p class="my-2 max-w-2xl text-sm">
    <strong>API key</strong> sends the API key of Organizr to the service, which has all the rights of
    an Admin. Switch it on only for a service behind the forward auth of Organizr, at an address inside
    the network.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  {#if rows}
    <form onsubmit={save}>
      <div class="overflow-x-auto">
        <table class="table-sm table" aria-label="Services of the health checks">
          <thead>
            <tr>
              <th scope="col">Name</th>
              <th scope="col">Id of the check</th>
              <th scope="col">Address inside the network</th>
              <th scope="col">Address from the internet</th>
              <th scope="col">On</th>
              <th scope="col">API key</th>
              <th scope="col"><span class="sr-only">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {#each rows as row, index (index)}
              {@const what = row.name || `service ${index + 1}`}
              <tr>
                <td>
                  <input
                    class="input input-sm w-36"
                    aria-label="Name of service {index + 1}"
                    bind:value={row.name}
                  />
                </td>
                <td>
                  <input
                    class="input input-sm w-44"
                    aria-label="Id of the check of {what}"
                    bind:value={row.uuid}
                  />
                </td>
                <td>
                  <input
                    class="input input-sm w-52"
                    type="url"
                    aria-label="Address inside the network of {what}"
                    bind:value={row.internal}
                  />
                </td>
                <td>
                  <input
                    class="input input-sm w-52"
                    type="url"
                    aria-label="Address from the internet of {what}"
                    bind:value={row.external}
                  />
                </td>
                <td>
                  <input
                    type="checkbox"
                    class="toggle toggle-sm"
                    aria-label="Check {what}"
                    bind:checked={row.enabled}
                  />
                </td>
                <td>
                  <input
                    type="checkbox"
                    class="toggle toggle-sm"
                    aria-label="Send the API key to {what}"
                    bind:checked={row.sendKey}
                  />
                </td>
                <td>
                  <button
                    type="button"
                    class="btn btn-ghost btn-xs"
                    aria-label="Remove {what}"
                    onclick={() => (rows = (rows ?? []).filter((_, place) => place !== index))}
                  >
                    Remove
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class="mt-2 flex flex-wrap gap-2">
        <button
          type="button"
          class="btn btn-sm"
          onclick={() => (rows = [...(rows ?? []), emptyRow()])}
        >
          Add a service
        </button>
        <button type="button" class="btn btn-sm" disabled={busy} onclick={importNow}>
          Import the checks of Healthchecks
        </button>
        <button type="submit" class="btn btn-sm btn-primary" disabled={busy || !hasChanges}>
          Save the services
        </button>
        <button type="button" class="btn btn-sm" disabled={busy || hasChanges} onclick={runNow}>
          {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
          Run the checks now
        </button>
      </div>
    </form>
  {/if}
  {#if results}
    <ul class="mt-4 text-sm" aria-label="Results of the run">
      {#each Object.entries(results) as [key, service] (key)}
        <li>
          <span class="font-medium">{service['Service Name'] || key}</span>:
          {#each Object.entries(service.results ?? {}) as [which, result] (which)}
            <span
              class="badge badge-sm {result === 'Success'
                ? 'badge-success'
                : result === 'Error'
                  ? 'badge-error'
                  : 'badge-ghost'} ml-1"
            >
              {which === 'internal' ? 'inside' : 'internet'}: {result}
            </span>
          {:else}
            <span class="opacity-70">no address</span>
          {/each}
        </li>
      {/each}
    </ul>
  {/if}
</section>
