<script lang="ts">
  // The editor of the bookmark plugin: its categories and its bookmarks.
  import { onMount } from 'svelte';

  import Dialog from '../../components/Dialog.svelte';
  import Picture from '../../components/Picture.svelte';
  import PictureField from '../../components/PictureField.svelte';
  import { api, failureText } from '../../lib/api';

  interface Bookmark {
    id: number;
    order: number;
    category_id: number;
    name: string;
    url: string;
    enabled: number;
    group_id: number;
    image: string;
    background_color: string;
    text_color: string;
  }

  interface Category {
    id: number;
    order: number;
    category: string;
    category_id: number;
    default: number;
  }

  interface Group {
    group: string;
    group_id: number;
  }

  interface Lists {
    tabs: Bookmark[];
    categories: Category[];
    groups: Group[];
  }

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let lists = $state<Lists | null>(null);
  let error = $state<string | null>(null);
  /** The bookmark in the dialog: one to change, or `'new'`. */
  let editing = $state<Bookmark | 'new' | null>(null);
  let form = $state({
    name: '',
    url: '',
    image: '',
    category_id: 0,
    group_id: 0,
    background_color: '#1f2937',
    text_color: '#ffffff',
    enabled: true,
  });
  let formError = $state<string | null>(null);
  let newCategory = $state('');
  let busy = $state(false);

  async function load() {
    try {
      lists = (await api<Lists>('GET', 'plugins/bookmark/tabs')).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Runs a change, loads the lists again, and says what was done. */
  async function run(change: () => Promise<unknown>, message: string): Promise<string | null> {
    busy = true;
    try {
      await change();
      await load();
      ondone(message);
      return null;
    } catch (failure) {
      return failureText(failure);
    } finally {
      busy = false;
    }
  }

  async function act(change: () => Promise<unknown>, message: string) {
    error = await run(change, message);
  }

  function open(bookmark: Bookmark | 'new') {
    const first = lists?.categories.find((category) => category.default === 1);
    form =
      bookmark === 'new'
        ? {
            name: '',
            url: '',
            image: '',
            category_id: first?.category_id ?? lists?.categories[0]?.category_id ?? 0,
            group_id: 999,
            background_color: '#1f2937',
            text_color: '#ffffff',
            enabled: true,
          }
        : {
            name: bookmark.name,
            url: bookmark.url,
            image: bookmark.image,
            category_id: bookmark.category_id,
            group_id: bookmark.group_id,
            background_color: bookmark.background_color,
            text_color: bookmark.text_color,
            enabled: bookmark.enabled === 1,
          };
    formError = null;
    editing = bookmark;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const target = editing;
    if (!target) {
      return;
    }
    const fields = { ...form, enabled: form.enabled ? 1 : 0 };
    formError =
      target === 'new'
        ? await run(() => api('POST', 'plugins/bookmark/tabs', fields), 'The bookmark is added.')
        : await run(
            () => api('PUT', `plugins/bookmark/tabs/${target.id}`, fields),
            'The bookmark is changed.',
          );
    if (formError === null) {
      editing = null;
    }
  }

  async function addCategory(event: SubmitEvent) {
    event.preventDefault();
    const name = newCategory;
    error = await run(
      () => api('POST', 'plugins/bookmark/categories', { category: name }),
      'The category is added.',
    );
    if (error === null) {
      newCategory = '';
    }
  }

  function categoryName(number: number): string {
    return (
      lists?.categories.find((category) => category.category_id === number)?.category ??
      'No category'
    );
  }

  onMount(load);
</script>

<section class="mt-8" aria-labelledby="bookmarks-title">
  <h2 id="bookmarks-title" class="text-base font-semibold">Bookmarks</h2>
  <p class="my-2 max-w-2xl text-sm">
    The bookmarks show on a page of their own. To put that page into the menu, select “Make the tab
    Bookmarks”.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  {#if lists}
    <div class="mb-4 flex flex-wrap gap-2">
      <button
        type="button"
        class="btn btn-sm"
        disabled={busy}
        onclick={() =>
          act(() => api('GET', 'plugins/bookmark/setup/tab'), 'The menu has the tab Bookmarks.')}
      >
        Make the tab Bookmarks
      </button>
      <button
        type="button"
        class="btn btn-sm btn-primary"
        disabled={busy || lists.categories.length === 0}
        onclick={() => open('new')}
      >
        Add a bookmark
      </button>
    </div>

    <h3 class="font-medium">Categories</h3>
    <ul class="my-2 flex flex-wrap gap-2" aria-label="Categories of the bookmarks">
      {#each lists.categories as category (category.id)}
        <li class="badge badge-lg gap-2">
          {category.category}
          {#if category.default === 1}
            <span class="text-xs opacity-70">default</span>
          {:else}
            <button
              type="button"
              class="link text-xs"
              disabled={busy}
              aria-label="Make {category.category} the default"
              onclick={() =>
                act(
                  () => api('PUT', `plugins/bookmark/categories/${category.id}`, { default: 1 }),
                  'The default category is changed.',
                )}
            >
              default
            </button>
          {/if}
          <button
            type="button"
            class="link text-xs"
            disabled={busy}
            aria-label="Delete the category {category.category}"
            onclick={() =>
              act(
                () => api('DELETE', `plugins/bookmark/categories/${category.id}`),
                'The category is deleted.',
              )}
          >
            delete
          </button>
        </li>
      {/each}
    </ul>
    <form class="mb-6 flex gap-2" onsubmit={addCategory}>
      <input
        class="input input-sm w-52"
        aria-label="Name of a new category"
        bind:value={newCategory}
        required
        maxlength="50"
      />
      <button type="submit" class="btn btn-sm" disabled={busy}>Add a category</button>
    </form>

    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Bookmarks">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Category</th>
            <th scope="col">Address</th>
            <th scope="col">Shown</th>
            <th scope="col"><span class="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          {#each lists.tabs as bookmark (bookmark.id)}
            <tr>
              <td>
                <span class="flex items-center gap-2">
                  <Picture image={bookmark.image} name={bookmark.name} />
                  {bookmark.name}
                </span>
              </td>
              <td>{categoryName(bookmark.category_id)}</td>
              <td class="max-w-xs truncate">{bookmark.url}</td>
              <td>{bookmark.enabled === 1 ? 'Yes' : 'No'}</td>
              <td class="whitespace-nowrap">
                <button
                  type="button"
                  class="btn btn-ghost btn-xs"
                  aria-label="Change {bookmark.name}"
                  onclick={() => open(bookmark)}
                >
                  Change
                </button>
                <button
                  type="button"
                  class="btn btn-ghost btn-xs"
                  disabled={busy}
                  aria-label="Delete {bookmark.name}"
                  onclick={() =>
                    act(
                      () => api('DELETE', `plugins/bookmark/tabs/${bookmark.id}`),
                      'The bookmark is deleted.',
                    )}
                >
                  Delete
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>

{#if editing && lists}
  <Dialog
    title={editing === 'new' ? 'Add a bookmark' : `Change the bookmark ${editing.name}`}
    onclose={() => (editing = null)}
  >
    <form class="mt-2" onsubmit={save}>
      <fieldset class="fieldset">
        <label class="label" for="bookmark-name">Name</label>
        <input
          id="bookmark-name"
          class="input w-full"
          bind:value={form.name}
          required
          maxlength="50"
        />

        <label class="label" for="bookmark-url">Address</label>
        <input id="bookmark-url" class="input w-full" type="url" bind:value={form.url} required />

        <label class="label" for="bookmark-image">Picture</label>
        <PictureField id="bookmark-image" bind:value={form.image} name={form.name} />

        <label class="label" for="bookmark-category">Category</label>
        <select id="bookmark-category" class="select w-full" bind:value={form.category_id}>
          {#each lists.categories as category (category.id)}
            <option value={category.category_id}>{category.category}</option>
          {/each}
        </select>

        <label class="label" for="bookmark-group">For the group, and groups with more access</label>
        <select id="bookmark-group" class="select w-full" bind:value={form.group_id}>
          {#each lists.groups as group (group.group_id)}
            <option value={group.group_id} translate="no">{group.group}</option>
          {/each}
        </select>

        <label class="label" for="bookmark-background">Colour of the tile</label>
        <input id="bookmark-background" type="color" bind:value={form.background_color} />

        <label class="label" for="bookmark-text">Colour of the name</label>
        <input id="bookmark-text" type="color" bind:value={form.text_color} />

        <label class="label mt-2 cursor-pointer justify-start gap-3">
          <input type="checkbox" class="toggle" bind:checked={form.enabled} />
          Show the bookmark on the page
        </label>
      </fieldset>
      {#if formError}
        <div role="alert" class="alert alert-error mt-2"><span>{formError}</span></div>
      {/if}
      <div class="modal-action">
        <button type="button" class="btn" onclick={() => (editing = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary" disabled={busy}>Save</button>
      </div>
    </form>
  </Dialog>
{/if}
