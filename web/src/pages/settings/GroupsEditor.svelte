<script lang="ts">
  // The table of all groups, with the ways to add, change and delete them.
  import Picture from '../../components/Picture.svelte';
  import { api } from '../../lib/api';
  import { isFixedGroup } from '../../lib/editor';
  import type { EditorData, Group } from '../../lib/types';
  import GroupDialog from './GroupDialog.svelte';

  let {
    data,
    change,
  }: {
    data: EditorData;
    /** Runs a change on the server and loads the data again. Returns the failure, or `null`. */
    change: (request: () => Promise<unknown>, done: string) => Promise<string | null>;
  } = $props();

  // The server lists the groups from the most access to the least.
  const groups = $derived(data.groups);

  /** How many users the group has. */
  function userCount(group: Group): number {
    return data.users.filter((user) => user.group_id === group.group_id).length;
  }

  // The dialog: closed, open for a new group, or open for one that is changed.
  let dialog = $state<{ group: Group | null } | null>(null);
  let error = $state<string | null>(null);

  async function run(request: () => Promise<unknown>, done: string) {
    error = await change(request, done);
  }

  async function save(fields: Record<string, unknown>): Promise<string | null> {
    const group = dialog?.group;
    const failure = await change(
      () => (group ? api('PUT', `groups/${group.id}`, fields) : api('POST', 'groups', fields)),
      group ? 'The group is changed.' : 'The group is added.',
    );
    if (failure === null) {
      dialog = null;
    }
    return failure;
  }

  function remove(group: Group) {
    if (confirm(`Delete the group ${group.group}?`)) {
      void run(() => api('DELETE', `groups/${group.id}`), 'The group is deleted.');
    }
  }
</script>

{#if error}
  <div role="alert" class="alert alert-error mb-4">
    <span>{error}</span>
  </div>
{/if}

<section class="org-panel panel panel-default" aria-labelledby="manage-groups-title">
  <div class="org-panel-heading panel-heading">
    <h2 id="manage-groups-title" class="uppercase">Manage Groups</h2>
    <button
      class="org-round-button org-round-info org-round-small"
      aria-label="Add a group"
      title="Add a group"
      onclick={() => (dialog = { group: null })}
    >
      <i class="oi-themify ti-plus" aria-hidden="true"></i>
    </button>
  </div>
  <div class="overflow-x-auto">
    <table class="org-table">
      <thead>
        <tr>
          <th scope="col" class="text-center">#</th>
          <th scope="col">Group Name</th>
          <th scope="col" class="text-center">Users</th>
          <th scope="col" class="text-center">Default</th>
          <th scope="col" class="text-center">Edit</th>
          <th scope="col" class="text-center">Delete</th>
        </tr>
      </thead>
      <tbody>
        {#each groups as group (group.id)}
          <tr>
            <td class="org-table-picture text-center">
              <Picture image={group.image} name={group.group} />
            </td>
            <th scope="row" translate="no">{group.group}</th>
            <td class="text-center">{userCount(group)}</td>
            <td class="text-center">
              <!-- The groups of the Admin and of guests are never the group for new users. -->
              <button
                class="org-round-button {group.default === 1
                  ? 'org-round-info'
                  : 'org-round-warning'}"
                aria-label={`Put new users into ${group.group}`}
                aria-pressed={group.default === 1}
                disabled={group.default === 1 || isFixedGroup(group)}
                onclick={() =>
                  run(
                    () => api('PUT', `groups/${group.id}`, { default: 1 }),
                    'The group is changed.',
                  )}
              >
                <i
                  class="oi-simpleline {group.default === 1
                    ? 'icon-user-following'
                    : 'icon-user-follow'}"
                  aria-hidden="true"
                ></i>
              </button>
            </td>
            <td class="text-center">
              <button class="org-round-button org-round-info" onclick={() => (dialog = { group })}>
                <i class="oi-themify ti-pencil-alt" aria-hidden="true"></i>
                <span class="sr-only">Change {group.group}</span>
              </button>
            </td>
            <td class="text-center">
              <button
                class="org-round-button org-round-danger"
                disabled={isFixedGroup(group)}
                onclick={() => remove(group)}
              >
                <i class="oi-themify ti-trash" aria-hidden="true"></i>
                <span class="sr-only">Delete {group.group}</span>
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>

{#if dialog}
  {#key dialog.group?.id}
    <GroupDialog group={dialog.group} onsave={save} onclose={() => (dialog = null)} />
  {/key}
{/if}
