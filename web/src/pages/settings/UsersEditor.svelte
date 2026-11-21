<script lang="ts">
  // The table of all users, with the ways to add, change, lock and delete them.
  import Picture from '../../components/Picture.svelte';
  import { api } from '../../lib/api';
  import { asSwitch } from '../../lib/editor';
  import { session } from '../../lib/session.svelte';
  import type { Account, EditorData } from '../../lib/types';
  import UserDialog from './UserDialog.svelte';

  let {
    data,
    change,
    part = 'manage',
  }: {
    data: EditorData;
    /** The table of the users, or the import of the users of a media server. */
    part?: 'manage' | 'import';
    /** Runs a change on the server and loads the data again. Returns the failure, or `null`. */
    change: (request: () => Promise<unknown>, done: string) => Promise<string | null>;
  } = $props();

  // The dialog: closed, open for a new user, or open for one that is changed.
  let dialog = $state<{ user: Account | null } | null>(null);
  let error = $state<string | null>(null);

  /** Whether the row is the account of the person who looks at the page. */
  function isOwn(user: Account): boolean {
    return session.launch?.user?.userID === user.id;
  }

  /** The day a user was stored, without the time. */
  function dayOf(user: Account): string {
    return user.register_date.slice(0, 10);
  }

  async function run(request: () => Promise<unknown>, done: string) {
    error = await change(request, done);
  }

  async function save(fields: Record<string, unknown>): Promise<string | null> {
    const user = dialog?.user;
    const failure = await change(
      () => (user ? api('PUT', `users/${user.id}`, fields) : api('POST', 'users', fields)),
      user ? 'The user is changed.' : 'The user is added.',
    );
    if (failure === null) {
      dialog = null;
    }
    return failure;
  }

  function setLocked(user: Account, locked: boolean) {
    void run(
      () => api('PUT', `users/${user.id}`, { locked: asSwitch(locked) }),
      locked ? 'The user is locked.' : 'The user is unlocked.',
    );
  }

  // The media servers that the users can come from, as the route names them.
  const IMPORTS = [
    { type: 'plex', name: 'Plex' },
    { type: 'jellyfin', name: 'Jellyfin' },
    { type: 'emby', name: 'Emby' },
  ] as const;
  let importing = $state(false);
  // What the server said about the last import: how many users it added.
  let imported = $state<string | null>(null);

  async function importFrom(type: string, name: string) {
    importing = true;
    imported = null;
    let message: string | null = null;
    error = await change(async () => {
      message = (await api<null>('POST', `users/import/${type}`, { type })).message;
    }, `The users of ${name} are imported.`);
    imported = error === null ? message : null;
    importing = false;
  }

  function remove(user: Account) {
    if (confirm(`Delete the user ${user.username}?`)) {
      void run(() => api('DELETE', `users/${user.id}`), 'The user is deleted.');
    }
  }
</script>

{#if error}
  <div role="alert" class="alert alert-error mb-4">
    <span>{error}</span>
  </div>
{/if}

{#if part === 'import'}
  <section class="org-panel panel panel-default" aria-labelledby="import-users">
    <div class="org-panel-heading panel-heading">
      <h2 id="import-users" class="uppercase">Import Users</h2>
    </div>
    <div class="org-panel-body panel-body">
      <p class="mb-4 text-sm">
        Adds an account for each user of the media server who has none. The server must be set up in
        the settings. Each new account gets a random password, so its user logs in through the media
        server.
      </p>
      <div class="flex flex-wrap items-center gap-2">
        {#each IMPORTS as server (server.type)}
          <button
            class="btn btn-info btn-sm"
            disabled={importing}
            onclick={() => void importFrom(server.type, server.name)}
          >
            Import {server.name} users
          </button>
        {/each}
        {#if imported}
          <span class="text-sm">{imported}</span>
        {/if}
      </div>
    </div>
  </section>
{:else}
  <section class="org-panel panel panel-default" aria-labelledby="manage-users-title">
    <div class="org-panel-heading panel-heading">
      <h2 id="manage-users-title" class="uppercase">Manage Users</h2>
      <button
        class="org-round-button org-round-info org-round-small"
        aria-label="Add a user"
        title="Add a user"
        onclick={() => (dialog = { user: null })}
      >
        <i class="oi-themify ti-plus" aria-hidden="true"></i>
      </button>
    </div>
    <div class="overflow-x-auto">
      <table class="org-table org-table-grid">
        <thead>
          <tr>
            <th scope="col" class="text-center">Avatar</th>
            <th scope="col">Username</th>
            <th scope="col">Email</th>
            <th scope="col" class="text-center">Date Registered</th>
            <th scope="col" class="text-center">Group</th>
            <th scope="col" class="text-center">Locked</th>
            <th scope="col" class="text-center">Password</th>
            <th scope="col" class="text-center">Action</th>
          </tr>
        </thead>
        <tbody>
          {#each data.users as user (user.id)}
            <tr>
              <td class="text-center">
                <span class="org-avatar"><Picture image={user.image} name={user.username} /></span>
              </td>
              <th scope="row" translate="no">{user.username}</th>
              <td>{user.email ?? ''}</td>
              <td class="text-center whitespace-nowrap">{dayOf(user)}</td>
              <td class="text-center">{user.group ?? ''}</td>
              <td class="text-center">
                <!-- Nobody locks the own account here: the way back in would be closed. -->
                <label class="inline-flex items-center gap-2">
                  <input
                    type="checkbox"
                    class="org-table-check"
                    aria-label={`Lock ${user.username}`}
                    checked={user.locked === 1}
                    disabled={isOwn(user)}
                    onchange={(event) => setLocked(user, event.currentTarget.checked)}
                  />
                  <span aria-hidden="true">{user.locked === 1 ? 'Yes' : 'No'}</span>
                </label>
              </td>
              <td class="text-center">
                <button class="org-icon-button" onclick={() => (dialog = { user })}>
                  <i class="oi-simpleline icon-user-following" aria-hidden="true"></i>
                  <span class="sr-only">Change {user.username}</span>
                </button>
              </td>
              <td class="text-center">
                <button
                  class="org-icon-button org-icon-danger"
                  disabled={isOwn(user)}
                  onclick={() => remove(user)}
                >
                  <i class="oi-themify ti-trash" aria-hidden="true"></i>
                  <span class="sr-only">Delete {user.username}</span>
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
{/if}

{#if dialog}
  {#key dialog.user?.id}
    <UserDialog
      user={dialog.user}
      groups={data.groups}
      onsave={save}
      onclose={() => (dialog = null)}
    />
  {/key}
{/if}
