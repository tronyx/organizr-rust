<script lang="ts">
  // The extensions in the folder plugins of the data folder: an Admin approves one,
  // switches one off, and deletes one.
  import { onMount } from 'svelte';

  import Dialog from '../../components/Dialog.svelte';
  import { api, ApiError, apiUrl, failureText } from '../../lib/api';
  import { session } from '../../lib/session.svelte';

  interface ExtensionRow {
    id: string;
    name: string;
    version: string;
    description: string;
    author: string;
    permissions: string[];
    least_access: number;
    sha256: string;
    enabled: boolean;
    approved: boolean;
    changed: boolean;
    problem: string | null;
  }

  /** An extension that the marketplace offers. */
  interface MarketRow {
    id: string;
    name: string;
    version: string;
    description: string;
    author: string;
    installed: boolean;
    up_to_date: boolean;
    problem: string | null;
  }

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  /** `null` until the list is loaded, and when this server has no extensions. */
  let rows = $state<ExtensionRow[] | null>(null);
  let error = $state<string | null>(null);
  /** `null` when this server has no marketplace, or the marketplace gave no list. */
  let market = $state<MarketRow[] | null>(null);
  let marketError = $state<string | null>(null);
  let busy = $state(false);
  /** The extension whose approval the dialog asks for. */
  let asking = $state<ExtensionRow | null>(null);

  /** Only the Admin approves, switches off and deletes. A Co-Admin sees the list. */
  const canChange = $derived(session.launch?.user?.groupID === 0);

  /** What each permission lets an extension do. */
  const PERMISSIONS: Record<string, string> = {
    storage: 'Keep values of its own in the database of Organizr',
    'http-local': 'Reach the sites that it names also when they are on your local network',
  };

  /** The text of a permission. `http:<host>` names a site that the extension may reach. */
  function permissionText(permission: string): string {
    if (permission.startsWith('http:')) {
      return `Send requests to the site ${permission.slice('http:'.length)}`;
    }
    return PERMISSIONS[permission] ?? permission;
  }

  async function load() {
    try {
      rows = (await api<ExtensionRow[]>('GET', 'extensions')).data;
      error = null;
    } catch (failure) {
      // A server that was built without extensions has no such route.
      if (failure instanceof ApiError && failure.status === 404) {
        rows = null;
      } else {
        error = failureText(failure);
      }
    }
  }

  async function loadMarket() {
    try {
      market = (await api<MarketRow[]>('GET', 'extensions/marketplace')).data;
      marketError = null;
    } catch (failure) {
      market = null;
      // Without a marketplace, the server says so, and the page shows no list.
      const none =
        failure instanceof ApiError &&
        (failure.status === 404 || failure.message === 'No marketplace is set up');
      marketError = none ? null : failureText(failure);
    }
  }

  function marketState(row: MarketRow): string {
    if (row.problem) {
      return row.problem;
    }
    if (!row.installed) {
      return 'Not installed';
    }
    return row.up_to_date ? 'Installed' : 'Another file is installed';
  }

  async function act(change: () => Promise<unknown>, message: string) {
    busy = true;
    try {
      await change();
      await Promise.all([load(), loadMarket()]);
      error = null;
      ondone(message);
    } catch (failure) {
      error = failureText(failure);
      await load();
    } finally {
      busy = false;
      asking = null;
    }
  }

  function approve(row: ExtensionRow) {
    return act(
      () =>
        api('POST', `extensions/${row.id}/enable`, {
          sha256: row.sha256,
          permissions: row.permissions,
        }),
      `The extension ${row.name} is approved and switched on.`,
    );
  }

  function stateOf(row: ExtensionRow): string {
    if (row.problem) {
      return row.problem;
    }
    if (row.changed) {
      return 'The file changed after the approval. The extension does not run.';
    }
    if (row.enabled) {
      return 'Switched on';
    }
    return row.approved ? 'Switched off' : 'Not approved';
  }

  function levelText(level: number): string {
    if (level >= 999) {
      return 'Everybody, also without a login';
    }
    return level === 0 ? 'The Admin' : `Groups of the level ${level} and above`;
  }

  onMount(() => {
    void load();
    void loadMarket();
  });
</script>

{#if rows || error}
  <section class="mt-8" aria-labelledby="extensions-title">
    <h2 id="extensions-title" class="text-base font-semibold">Extensions</h2>
    <p class="my-2 max-w-2xl text-sm">
      An extension is a program of another author in the folder plugins of the data folder. It runs
      in a sandbox: it has no files and no network. It answers requests only after the Admin
      approved it.
    </p>
    {#if error}
      <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}
    {#if rows && rows.length === 0}
      <p class="text-sm">There are no extensions in the folder.</p>
    {:else if rows}
      <div class="overflow-x-auto">
        <table class="table-sm table" aria-label="Extensions">
          <thead>
            <tr>
              <th scope="col">Extension</th>
              <th scope="col">For</th>
              <th scope="col">State</th>
              <th scope="col"><span class="sr-only">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {#each rows as row (row.id)}
              <tr>
                <td>
                  <span class="font-medium">{row.name}</span>
                  {row.version}
                  {#if row.author}<span class="text-xs">by {row.author}</span>{/if}
                  <br /><span class="text-xs">{row.description}</span>
                </td>
                <td>{levelText(row.least_access)}</td>
                <td>{stateOf(row)}</td>
                <td class="whitespace-nowrap">
                  {#if row.enabled}
                    <a
                      class="btn btn-ghost btn-xs"
                      href={apiUrl(`extension/${row.id}/`)}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      Open
                    </a>
                  {/if}
                  {#if canChange}
                    {#if row.enabled}
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        disabled={busy}
                        aria-label="Switch {row.name} off"
                        onclick={() =>
                          act(
                            () => api('POST', `extensions/${row.id}/disable`),
                            `The extension ${row.name} is switched off.`,
                          )}
                      >
                        Switch off
                      </button>
                    {:else if !row.problem}
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        disabled={busy}
                        aria-label="Approve {row.name}"
                        onclick={() => (asking = row)}
                      >
                        Approve
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="btn btn-ghost btn-xs"
                      disabled={busy}
                      aria-label="Delete {row.name}"
                      onclick={() =>
                        act(
                          () => api('DELETE', `extensions/${row.id}`),
                          `The extension ${row.name} is deleted.`,
                        )}
                    >
                      Delete
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    {#if marketError}
      <div role="alert" class="alert alert-warning mt-4"><span>{marketError}</span></div>
    {/if}
    {#if market}
      <h3 class="mt-6 font-medium">Marketplace</h3>
      <p class="my-2 max-w-2xl text-sm">
        An install downloads the file of an extension and makes sure that it is the file that the
        marketplace names. The extension then waits for your approval in the list above.
      </p>
      {#if market.length === 0}
        <p class="text-sm">The marketplace offers no extensions.</p>
      {:else}
        <div class="overflow-x-auto">
          <table class="table-sm table" aria-label="Extensions of the marketplace">
            <thead>
              <tr>
                <th scope="col">Extension</th>
                <th scope="col">State</th>
                <th scope="col"><span class="sr-only">Actions</span></th>
              </tr>
            </thead>
            <tbody>
              {#each market as row (row.id)}
                <tr>
                  <td>
                    <span class="font-medium">{row.name}</span>
                    {row.version}
                    {#if row.author}<span class="text-xs">by {row.author}</span>{/if}
                    <br /><span class="text-xs">{row.description}</span>
                  </td>
                  <td>{marketState(row)}</td>
                  <td>
                    {#if canChange && !row.problem && !(row.installed && row.up_to_date)}
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        disabled={busy}
                        aria-label="Install {row.name}"
                        onclick={() =>
                          act(
                            () => api('POST', `extensions/marketplace/${row.id}`),
                            `The extension ${row.name} is installed. Approve it in the list above.`,
                          )}
                      >
                        Install
                      </button>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  </section>
{/if}

{#if asking}
  {@const row = asking}
  <Dialog title="Approve the extension {row.name}" onclose={() => (asking = null)}>
    <p class="mt-2 text-sm">
      Approve an extension only when you trust its author. The approval is for this file. When the
      file changes, the extension stops until you approve it again.
    </p>
    <h3 class="mt-3 font-medium">What the extension can do</h3>
    <ul class="list-disc pl-5 text-sm" aria-label="Permissions of the extension">
      <li>Answer requests of: {levelText(row.least_access)}</li>
      <li>See the user name and the level of each caller</li>
      <li>Write lines to the log</li>
      {#each row.permissions as permission (permission)}
        <li>{permissionText(permission)}</li>
      {/each}
    </ul>
    <p class="mt-3 text-xs break-all">SHA-256 of the file: {row.sha256}</p>
    <div class="modal-action">
      <button type="button" class="btn" onclick={() => (asking = null)}>Cancel</button>
      <button type="button" class="btn btn-primary" disabled={busy} onclick={() => approve(row)}>
        Approve and switch on
      </button>
    </div>
  </Dialog>
{/if}
