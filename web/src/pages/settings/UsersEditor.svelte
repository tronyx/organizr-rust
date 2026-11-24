<script lang="ts">
  // The table of all users, with the ways to add, change, lock and delete them.
  import Pager from '../../components/Pager.svelte';
  import Picture from '../../components/Picture.svelte';
  import { api } from '../../lib/api';
  import { asSwitch } from '../../lib/editor';
  import {
    PAGE_SIZES,
    pageCount,
    pageOf,
    readPageSize,
    savePageSize,
    shownPage,
  } from '../../lib/paging';
  import { session } from '../../lib/session.svelte';
  import { localDateTime } from '../../lib/time';
  import type { Account, EditorData } from '../../lib/types';
  import { nextSort, sortUsers, type UserSort, type UserSortKey } from '../../lib/userSort';
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

  // The dialog: closed, open for a new user, or open for one that is changed. `password`
  // is set when the key of the Password column opened it: it starts in the new password.
  let dialog = $state<{ user: Account | null; password?: boolean } | null>(null);
  let error = $state<string | null>(null);

  // The users a page at a time, as in PHP. The page stays when the users load again after
  // a change; after a delete that empties the last page, the page before it shows.
  let pageSize = $state(readPageSize('users'));
  let page = $state(1);
  const pages = $derived(pageCount(data.users.length, pageSize));
  const current = $derived(shownPage(page, data.users.length, pageSize));
  // A click on the heading of a column sorts the users by it, as in PHP. Before that the
  // users keep the order of the server.
  let sort = $state<UserSort | null>(null);
  const sorted = $derived(sort ? sortUsers(data.users, sort) : data.users);
  const shown = $derived(pageOf(sorted, current, pageSize));

  function sortBy(key: UserSortKey) {
    sort = nextSort(sort, key);
    page = 1;
  }

  /** The state of a sortable column, for `aria-sort`. */
  function sortState(key: UserSortKey): 'ascending' | 'descending' | undefined {
    if (sort?.key !== key) {
      return undefined;
    }
    return sort.ascending ? 'ascending' : 'descending';
  }

  function setPageSize(size: number) {
    // The first user of the page stays in view.
    const first = (current - 1) * pageSize;
    pageSize = size;
    page = Math.floor(first / size) + 1;
    savePageSize('users', size);
  }

  /** Whether the row is the account of the person who looks at the page. */
  function isOwn(user: Account): boolean {
    return session.launch?.user?.userID === user.id;
  }

  /** When a user was stored, with the time, in the local time of the browser. */
  function registered(user: Account): string {
    return localDateTime(user.register_date);
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

  /** A click on a row opens its user, as in PHP, unless it was on a control of the row or
   *  ended the selection of a text (to copy an address, for example). */
  function openRow(event: MouseEvent, user: Account) {
    const target = event.target as Element | null;
    if (target?.closest('button, input, label, a, select, textarea')) {
      return;
    }
    if ((window.getSelection()?.toString() ?? '') !== '') {
      return;
    }
    dialog = { user };
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
  <section class="org-panel" aria-labelledby="import-users">
    <div class="org-panel-heading">
      <h2 id="import-users" class="uppercase">Import Users</h2>
    </div>
    <div class="org-panel-body">
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
  {#snippet sortable(key: UserSortKey, label: string, className: string)}
    {@const state = sortState(key)}
    <th scope="col" class={className || undefined} aria-sort={state}>
      <button class="org-sort-button" onclick={() => sortBy(key)}>
        {label}
        <span class="org-sort-mark" aria-hidden="true">
          {state === 'ascending' ? '▲' : state === 'descending' ? '▼' : ''}
        </span>
      </button>
    </th>
  {/snippet}
  <section class="org-panel" aria-labelledby="manage-users-title">
    <div class="org-panel-heading">
      <h2 id="manage-users-title" class="uppercase">Manage Users</h2>
      <!-- PHP hides the page size on a phone. -->
      <select
        class="org-form-control org-page-size hidden sm:block"
        aria-label="Users per page"
        value={pageSize}
        onchange={(event) => setPageSize(Number(event.currentTarget.value))}
      >
        {#each PAGE_SIZES as size (size)}
          <option value={size}>{size}</option>
        {/each}
      </select>
      <button
        class="org-round-button org-round-info org-round-small"
        aria-label="Add a user"
        title="Add a user"
        onclick={() => (dialog = { user: null })}
      >
        <i class="oi-themify ti-plus" aria-hidden="true"></i>
      </button>
    </div>
    <div class="org-users-scroll overflow-x-auto">
      <table class="org-table org-table-grid org-users-table">
        <thead>
          <tr>
            <th scope="col" class="org-users-avatar text-center">Avatar</th>
            {@render sortable('username', 'Username', '')}
            {@render sortable('email', 'Email', 'org-users-email')}
            {@render sortable('register_date', 'Date Registered', 'org-users-date text-center')}
            {@render sortable('group_id', 'Group', 'text-center')}
            {@render sortable('locked', 'Locked', 'text-center')}
            <th scope="col" class="text-center">Password</th>
            <th scope="col" class="text-center">Action</th>
          </tr>
        </thead>
        <tbody>
          {#each shown as user (user.id)}
            <!-- The click on a row is a shortcut for the mouse, as in PHP. The Edit button of
                 the row does the same for the keyboard, so the row needs no key or role. -->
            <tr class="org-click-row" onclick={(event) => openRow(event, user)}>
              <td class="org-users-avatar text-center">
                <span class="org-avatar">
                  <Picture image={user.image} name={user.username} avatar />
                </span>
              </td>
              <th scope="row" translate="no">
                {user.username}
                <!-- In a narrow card the column of the date is hidden: the date shows here. -->
                <span class="org-users-date-line">{registered(user)}</span>
              </th>
              <td class="org-users-email" title={user.email ?? undefined}>{user.email ?? ''}</td>
              <td class="org-users-date text-center whitespace-nowrap">{registered(user)}</td>
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
                <!-- The title is the name of the button and its tooltip: no other text names or
                     describes it, so a screen reader reads it once. -->
                <button
                  class="org-icon-button"
                  title={`Change the password of ${user.username}`}
                  onclick={() => (dialog = { user, password: true })}
                >
                  <i class="oi-materialize mdi-account-key" aria-hidden="true"></i>
                </button>
              </td>
              <td class="text-center">
                <span class="org-users-actions">
                  <button
                    class="org-icon-button"
                    title={`Edit ${user.username}`}
                    onclick={() => (dialog = { user })}
                  >
                    <i class="oi-themify ti-pencil-alt" aria-hidden="true"></i>
                  </button>
                  <button
                    class="org-icon-button org-icon-danger"
                    title={`Delete ${user.username}`}
                    disabled={isOwn(user)}
                    onclick={() => remove(user)}
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
    <Pager page={current} count={pages} label="Pages of the users" onpage={(to) => (page = to)} />
  </section>
{/if}

{#if dialog}
  {#key dialog.user?.id}
    <UserDialog
      user={dialog.user}
      groups={data.groups}
      focusPassword={dialog.password ?? false}
      onsave={save}
      onclose={() => (dialog = null)}
    />
  {/key}
{/if}
