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

{#if error}
  <div role="alert" class="alert alert-error mb-4">
    <span>{error}</span>
  </div>
{/if}

<section class="org-panel" aria-labelledby="category-editor-title">
  <div class="org-panel-heading">
    <h2 id="category-editor-title">Category Editor</h2>
    <button
      class="org-round-button org-round-info org-round-small"
      aria-label="Add a category"
      title="Add a category"
      onclick={() => (dialog = { category: null })}
    >
      <i class="oi-themify ti-plus" aria-hidden="true"></i>
    </button>
  </div>
  <div class="overflow-x-auto">
    <table class="org-table">
      <thead>
        <tr>
          <th scope="col">#</th>
          <th scope="col">Name</th>
          <th scope="col" class="text-center">Tabs</th>
          <th scope="col" class="text-center">Default</th>
          <th scope="col" class="text-center">Edit</th>
          <th scope="col" class="text-center">Delete</th>
        </tr>
      </thead>
      <tbody>
        {#each categories as category, index (category.id)}
          <tr>
            <td class="whitespace-nowrap">
              <span class="flex items-center gap-2">
                <span class="org-order">
                  <button
                    aria-label={`Move ${category.category} up`}
                    disabled={index === 0}
                    onclick={() => move(index, -1)}
                  >
                    <i class="oi-themify ti-angle-up" aria-hidden="true"></i>
                  </button>
                  <button
                    aria-label={`Move ${category.category} down`}
                    disabled={index === categories.length - 1}
                    onclick={() => move(index, 1)}
                  >
                    <i class="oi-themify ti-angle-down" aria-hidden="true"></i>
                  </button>
                </span>
                <span class="org-table-picture">
                  <Picture image={category.image} name={category.category} editor />
                </span>
              </span>
            </td>
            <th scope="row" translate="no">{category.category}</th>
            <td class="text-center">{tabCount(category)}</td>
            <td class="text-center">
              <button
                class="org-round-button {category.default === 1
                  ? 'org-round-info'
                  : 'org-round-warning'}"
                aria-label={`Put new tabs into ${category.category}`}
                aria-pressed={category.default === 1}
                disabled={category.default === 1}
                onclick={() =>
                  run(
                    () => api('PUT', `categories/${category.id}`, { default: 1 }),
                    'The category is changed.',
                  )}
              >
                <i
                  class="oi-simpleline {category.default === 1
                    ? 'icon-user-following'
                    : 'icon-user-follow'}"
                  aria-hidden="true"
                ></i>
              </button>
            </td>
            <td class="text-center">
              <button
                class="org-round-button org-round-info"
                onclick={() => (dialog = { category })}
              >
                <i class="oi-themify ti-pencil-alt" aria-hidden="true"></i>
                <span class="sr-only">Change {category.category}</span>
              </button>
            </td>
            <td class="text-center">
              <button class="org-round-button org-round-danger" onclick={() => remove(category)}>
                <i class="oi-themify ti-trash" aria-hidden="true"></i>
                <span class="sr-only">Delete {category.category}</span>
              </button>
            </td>
          </tr>
        {:else}
          <tr><td colspan="6">There is no category yet.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>

{#if dialog}
  {#key dialog.category?.id}
    <CategoryDialog category={dialog.category} onsave={save} onclose={() => (dialog = null)} />
  {/key}
{/if}
