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
  }: {
    data: EditorData;
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

<div class="mb-4 flex items-center justify-between gap-2">
  <p class="opacity-80">
    The people who can log in. The group of a user decides which tabs the user sees. A new user
    joins the group for new users.
  </p>
  <button class="btn btn-primary btn-sm" onclick={() => (dialog = { user: null })}>
    Add a user
  </button>
</div>

<section class="mb-4" aria-labelledby="import-users">
  <h2 id="import-users" class="font-semibold">Import users</h2>
  <p class="mb-2 text-sm opacity-80">
    Adds an account for each user of the media server who has none. The server must be set up in the
    settings. Each new account gets a random password, so its user logs in through the media server.
  </p>
  <div class="flex flex-wrap items-center gap-2">
    {#each IMPORTS as server (server.type)}
      <button
        class="btn btn-sm"
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
</section>

{#if error}
  <div role="alert" class="alert alert-error mb-4">
    <span>{error}</span>
  </div>
{/if}

<div class="overflow-x-auto">
  <table class="table">
    <thead>
      <tr>
        <th scope="col">Name</th>
        <th scope="col">E-mail address</th>
        <th scope="col">Group</th>
        <th scope="col">Since</th>
        <th scope="col">Locked</th>
        <th scope="col"><span class="sr-only">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each data.users as user (user.id)}
        <tr>
          <th scope="row">
            <span class="flex items-center gap-2 font-normal">
              <Picture image={user.image} name={user.username} />
              {user.username}
            </span>
          </th>
          <td>{user.email ?? ''}</td>
          <td>{user.group ?? ''}</td>
          <td class="whitespace-nowrap">{dayOf(user)}</td>
          <td>
            <!-- Nobody locks the own account here: the way back in would be closed. -->
            <input
              type="checkbox"
              class="toggle toggle-sm"
              aria-label={`Lock ${user.username}`}
              checked={user.locked === 1}
              disabled={isOwn(user)}
              onchange={(event) => setLocked(user, event.currentTarget.checked)}
            />
          </td>
          <td class="whitespace-nowrap text-right">
            <button class="btn btn-ghost btn-xs" onclick={() => (dialog = { user })}>
              Change<span class="sr-only"> {user.username}</span>
            </button>
            <button
              class="btn btn-ghost btn-xs text-error"
              disabled={isOwn(user)}
              onclick={() => remove(user)}
            >
              Delete<span class="sr-only"> {user.username}</span>
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

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
