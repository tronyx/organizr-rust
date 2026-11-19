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

<div class="mb-4 flex items-center justify-between gap-2">
  <p class="opacity-80">
    The groups of users, from the most access to the least. A tab is for the groups between two
    levels. The server gives a new group the level after the last one before “Guest”.
  </p>
  <button class="btn btn-primary btn-sm" onclick={() => (dialog = { group: null })}>
    Add a group
  </button>
</div>

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
        <th scope="col">Level</th>
        <th scope="col">For new users</th>
        <th scope="col"><span class="sr-only">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each groups as group (group.id)}
        <tr>
          <th scope="row">
            <span class="flex items-center gap-2 font-normal">
              <Picture image={group.image} name={group.group} />
              {group.group}
            </span>
          </th>
          <td>{group.group_id}</td>
          <td>
            <!-- The groups of the Admin and of guests are never the group for new users. -->
            <input
              type="radio"
              class="radio radio-sm"
              name="default-group"
              aria-label={`Put new users into ${group.group}`}
              checked={group.default === 1}
              disabled={isFixedGroup(group)}
              onchange={() =>
                run(
                  () => api('PUT', `groups/${group.id}`, { default: 1 }),
                  'The group is changed.',
                )}
            />
          </td>
          <td class="whitespace-nowrap text-right">
            <button class="btn btn-ghost btn-xs" onclick={() => (dialog = { group })}>
              Change<span class="sr-only"> {group.group}</span>
            </button>
            <button
              class="btn btn-ghost btn-xs text-error"
              disabled={isFixedGroup(group)}
              onclick={() => remove(group)}
            >
              Delete<span class="sr-only"> {group.group}</span>
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if dialog}
  {#key dialog.group?.id}
    <GroupDialog group={dialog.group} onsave={save} onclose={() => (dialog = null)} />
  {/key}
{/if}
