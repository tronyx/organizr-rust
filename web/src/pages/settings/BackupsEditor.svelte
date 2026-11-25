<script lang="ts">
  // The page Backup: the settings of the automatic backup above the backups of the install.
  // A backup can be made, saved to this computer and deleted. Admins and Co-Admins can use
  // them, as in PHP (D-74); anybody else is told so instead of the settings and the list.
  import { onMount } from 'svelte';

  import { api, apiUrl, failureText } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { BACKUP } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  interface Backup {
    name: string;
    size: string;
    date: string;
  }

  interface List {
    total_size: string;
    total_files: number;
    files: Backup[] | null;
  }

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let list = $state<List | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** Whether the person is an Admin or a Co-Admin, who may use the backups, as in PHP. */
  const mayBackUp = $derived((session.launch?.user?.groupID ?? Infinity) <= 1);

  async function load() {
    if (!mayBackUp) {
      return;
    }
    try {
      list = (await api<List>('GET', 'backup')).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function act(change: () => Promise<unknown>, message: string) {
    busy = true;
    try {
      await change();
      await load();
      ondone(message);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  /** The time of a backup as `2026-11-20 03:15 UTC`. */
  function dateText(date: string): string {
    return `${date.slice(0, 10)} ${date.slice(11, 16)} UTC`;
  }

  onMount(load);
</script>

{#if mayBackUp}
  <SettingsForm page={BACKUP} {ondone} />
{/if}

<section class="mt-8" aria-labelledby="backups-title">
  <h2 id="backups-title" class="text-base font-semibold">Backups</h2>
  <p class="my-2 max-w-2xl">
    A backup is a zip file with the pictures and the other files of the data folder. With SQLite, it
    also holds the database. With PostgreSQL, save the database with the tools of PostgreSQL. A
    backup never holds the key for the stored secrets: keep a copy of the file secret.key at a safe
    place.
  </p>
  {#if !mayBackUp}
    <div role="status" class="alert alert-info mb-4">
      <span>
        Only an Admin or a Co-Admin can make, download and delete backups, because a backup holds
        the password hashes of every user. You are signed in as {session.launch?.user?.group ??
          'a user'}.
      </span>
    </div>
  {/if}
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  {#if list}
    <button
      type="button"
      class="org-form-button mb-4"
      disabled={busy}
      onclick={() => act(() => api('POST', 'backup'), 'The backup is made.')}
    >
      {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
      Make a backup now
    </button>
    {#if list.files === null}
      <p>There are no backups yet.</p>
    {:else}
      <p>
        {list.total_files} backups, {list.total_size} in all.
      </p>
      <div class="overflow-x-auto">
        <table class="org-table org-table-grid" aria-label="Backups">
          <thead>
            <tr>
              <th scope="col">File</th>
              <th scope="col">Made</th>
              <th scope="col">Size</th>
              <th scope="col"><span class="sr-only">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {#each list.files as backup (backup.name)}
              <tr>
                <td class="font-mono">{backup.name}</td>
                <td class="whitespace-nowrap">{dateText(backup.date)}</td>
                <td class="whitespace-nowrap">{backup.size}</td>
                <td class="whitespace-nowrap">
                  <span class="inline-flex gap-2">
                    <a
                      class="org-round-button org-round-info"
                      href={apiUrl(`backup/${encodeURIComponent(backup.name)}`)}
                      download={backup.name}
                      aria-label="Save {backup.name} to this computer"
                      title="Save"
                    >
                      <i class="oi-themify ti-download" aria-hidden="true"></i>
                    </a>
                    <button
                      type="button"
                      class="org-round-button org-round-danger"
                      disabled={busy}
                      aria-label="Delete {backup.name}"
                      title="Delete"
                      onclick={() =>
                        act(
                          () => api('DELETE', `backup/${encodeURIComponent(backup.name)}`),
                          'The backup is deleted.',
                        )}
                    >
                      <i class="oi-themify ti-trash" aria-hidden="true"></i>
                    </button>
                  </span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</section>
