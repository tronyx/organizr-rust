<script lang="ts">
  // The form for a new group, or for a group that is changed.
  import Dialog from '../../components/Dialog.svelte';
  import type { Group } from '../../lib/types';

  let {
    group,
    onsave,
    onclose,
  }: {
    /** The group to change. `null` for a new group. */
    group: Group | null;
    /** Stores the fields. Returns why the server refused them, or `null`. */
    onsave: (fields: Record<string, unknown>) => Promise<string | null>;
    onclose: () => void;
  } = $props();

  // The form starts with the values of the group. The dialog is made anew for each one.
  // svelte-ignore state_referenced_locally
  let name = $state(group?.group ?? '');
  // svelte-ignore state_referenced_locally
  let image = $state(group?.image ?? '');
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = await onsave({ group: name, image });
    busy = false;
  }
</script>

<Dialog title={group ? `Change the group ${group.group}` : 'Add a group'} {onclose}>
  <form class="mt-2" onsubmit={save}>
    <fieldset class="fieldset">
      <label class="label" for="group-name">Name</label>
      <input id="group-name" class="input w-full" bind:value={name} required maxlength="50" />

      <label class="label" for="group-image">Picture</label>
      <input
        id="group-image"
        class="input w-full"
        bind:value={image}
        required
        placeholder="https://example.com/picture.png"
      />
    </fieldset>

    {#if error}
      <div role="alert" class="alert alert-error mt-2">
        <span>{error}</span>
      </div>
    {/if}

    <div class="modal-action">
      <button type="button" class="btn" onclick={onclose}>Cancel</button>
      <button class="btn btn-primary" disabled={busy}>Save</button>
    </div>
  </form>
</Dialog>
