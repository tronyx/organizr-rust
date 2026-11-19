<script lang="ts">
  // The form for a new category, or for a category that is changed.
  import Dialog from '../../components/Dialog.svelte';
  import type { Category } from '../../lib/types';

  let {
    category,
    onsave,
    onclose,
  }: {
    /** The category to change. `null` for a new category. */
    category: Category | null;
    /** Stores the fields. Returns why the server refused them, or `null`. */
    onsave: (fields: Record<string, unknown>) => Promise<string | null>;
    onclose: () => void;
  } = $props();

  // The form starts with the values of the category. The dialog is made anew for each one.
  // svelte-ignore state_referenced_locally
  let name = $state(category?.category ?? '');
  // svelte-ignore state_referenced_locally
  let image = $state(category?.image ?? '');
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = await onsave({ category: name, image });
    busy = false;
  }
</script>

<Dialog title={category ? `Change the category ${category.category}` : 'Add a category'} {onclose}>
  <form class="mt-2" onsubmit={save}>
    <fieldset class="fieldset">
      <label class="label" for="category-name">Name</label>
      <input id="category-name" class="input w-full" bind:value={name} required maxlength="50" />

      <label class="label" for="category-image">Picture</label>
      <input
        id="category-image"
        class="input w-full"
        bind:value={image}
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
