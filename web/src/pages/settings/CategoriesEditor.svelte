<script lang="ts">
  // The table of all categories, with the ways to add, change, order and delete them.
  import Picture from '../../components/Picture.svelte';
  import { api } from '../../lib/api';
  import { inMenuOrder, placesAfterMove } from '../../lib/editor';
  import type { Category, EditorData } from '../../lib/types';
  import CategoryDialog from './CategoryDialog.svelte';

  let {
    data,
    change,
  }: {
    data: EditorData;
    /** Runs a change on the server and loads the data again. Returns the failure, or `null`. */
    change: (request: () => Promise<unknown>, done: string) => Promise<string | null>;
  } = $props();

  const categories = $derived(inMenuOrder(data.categories));

  // The dialog: closed, open for a new category, or open for one that is changed.
  let dialog = $state<{ category: Category | null } | null>(null);
  let error = $state<string | null>(null);

  function tabCount(category: Category): number {
    return data.tabs.filter((tab) => tab.category_id === category.category_id).length;
  }

  async function run(request: () => Promise<unknown>, done: string) {
    error = await change(request, done);
  }

  async function save(fields: Record<string, unknown>): Promise<string | null> {
    const category = dialog?.category;
    const failure = await change(
      () =>
        category
          ? api('PUT', `categories/${category.id}`, fields)
          : api('POST', 'categories', fields),
      category ? 'The category is changed.' : 'The category is added.',
    );
    if (failure === null) {
      dialog = null;
    }
    return failure;
  }

  function move(index: number, step: -1 | 1) {
    const places = placesAfterMove(categories, index, step);
    if (places) {
      void run(() => api('PUT', 'categories', places), 'The order is changed.');
    }
  }

  function remove(category: Category) {
    if (confirm(`Delete the category ${category.category}?`)) {
      void run(() => api('DELETE', `categories/${category.id}`), 'The category is deleted.');
    }
  }
</script>

<div class="mb-4 flex items-center justify-between gap-2">
  <p class="opacity-80">
    The headings of the menu, in their order. The tabs of “Unsorted” stand in the menu without a
    heading.
  </p>
  <button class="btn btn-primary btn-sm" onclick={() => (dialog = { category: null })}>
    Add a category
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
        <th scope="col">Order</th>
        <th scope="col">Name</th>
        <th scope="col">Tabs</th>
        <th scope="col">For new tabs</th>
        <th scope="col"><span class="sr-only">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each categories as category, index (category.id)}
        <tr>
          <td class="whitespace-nowrap">
            <button
              class="btn btn-ghost btn-xs"
              aria-label={`Move ${category.category} up`}
              disabled={index === 0}
              onclick={() => move(index, -1)}
            >
              ↑
            </button>
            <button
              class="btn btn-ghost btn-xs"
              aria-label={`Move ${category.category} down`}
              disabled={index === categories.length - 1}
              onclick={() => move(index, 1)}
            >
              ↓
            </button>
          </td>
          <th scope="row">
            <span class="flex items-center gap-2 font-normal">
              <Picture image={category.image} name={category.category} />
              {category.category}
            </span>
          </th>
          <td>{tabCount(category)}</td>
          <td>
            <input
              type="radio"
              class="radio radio-sm"
              name="default-category"
              aria-label={`Put new tabs into ${category.category}`}
              checked={category.default === 1}
              onchange={() =>
                run(
                  () => api('PUT', `categories/${category.id}`, { default: 1 }),
                  'The category is changed.',
                )}
            />
          </td>
          <td class="whitespace-nowrap text-right">
            <button class="btn btn-ghost btn-xs" onclick={() => (dialog = { category })}>
              Change<span class="sr-only"> {category.category}</span>
            </button>
            <button class="btn btn-ghost btn-xs text-error" onclick={() => remove(category)}>
              Delete<span class="sr-only"> {category.category}</span>
            </button>
          </td>
        </tr>
      {:else}
        <tr><td colspan="5">There is no category yet.</td></tr>
      {/each}
    </tbody>
  </table>
</div>

{#if dialog}
  {#key dialog.category?.id}
    <CategoryDialog category={dialog.category} onsave={save} onclose={() => (dialog = null)} />
  {/key}
{/if}
