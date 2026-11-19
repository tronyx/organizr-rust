<script lang="ts">
  // The table of all tabs, with the ways to add, change, order and delete them.
  import Picture from '../../components/Picture.svelte';
  import { api, failureText } from '../../lib/api';
  import { asSwitch, inMenuOrder, placesAfterMove } from '../../lib/editor';
  import type { EditorData, Tab } from '../../lib/types';
  import TabDialog from './TabDialog.svelte';

  let {
    data,
    change,
  }: {
    data: EditorData;
    /** Runs a change on the server and loads the data again. Returns the failure, or `null`. */
    change: (request: () => Promise<unknown>, done: string) => Promise<string | null>;
  } = $props();

  const KINDS = ['Page of Organizr', 'Inside Organizr', 'New window'] as const;

  const tabs = $derived(inMenuOrder(data.tabs));

  // The dialog: closed, open for a new tab, or open for a tab that is changed.
  let dialog = $state<{ tab: Tab | null } | null>(null);
  let error = $state<string | null>(null);

  function categoryName(tab: Tab): string {
    return (
      data.categories.find((category) => category.category_id === tab.category_id)?.category ?? '—'
    );
  }

  function groupName(level: number): string {
    return data.groups.find((group) => group.group_id === level)?.group ?? String(level);
  }

  async function run(request: () => Promise<unknown>, done: string) {
    error = await change(request, done);
  }

  async function edit(tab: Tab) {
    // The list holds the addresses with their placeholders filled in. The form needs
    // them as they are stored.
    try {
      dialog = { tab: (await api<Tab>('GET', `tabs/${tab.id}`)).data };
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function save(fields: Record<string, unknown>): Promise<string | null> {
    const tab = dialog?.tab;
    const failure = await change(
      () => (tab ? api('PUT', `tabs/${tab.id}`, fields) : api('POST', 'tabs', fields)),
      tab ? 'The tab is changed.' : 'The tab is added.',
    );
    if (failure === null) {
      dialog = null;
    }
    return failure;
  }

  function move(index: number, step: -1 | 1) {
    const places = placesAfterMove(tabs, index, step);
    if (places) {
      void run(() => api('PUT', 'tabs', places), 'The order is changed.');
    }
  }

  function remove(tab: Tab) {
    if (confirm(`Delete the tab ${tab.name}?`)) {
      void run(() => api('DELETE', `tabs/${tab.id}`), 'The tab is deleted.');
    }
  }
</script>

<div class="mb-4 flex items-center justify-between gap-2">
  <p class="opacity-80">The tabs of the menu, in their order.</p>
  <button class="btn btn-primary btn-sm" onclick={() => (dialog = { tab: null })}>Add a tab</button>
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
        <th scope="col">Category</th>
        <th scope="col">Opens</th>
        <th scope="col">Shown to</th>
        <th scope="col">In the menu</th>
        <th scope="col">At the start</th>
        <th scope="col"><span class="sr-only">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each tabs as tab, index (tab.id)}
        <tr>
          <td class="whitespace-nowrap">
            <button
              class="btn btn-ghost btn-xs"
              aria-label={`Move ${tab.name} up`}
              disabled={index === 0}
              onclick={() => move(index, -1)}
            >
              ↑
            </button>
            <button
              class="btn btn-ghost btn-xs"
              aria-label={`Move ${tab.name} down`}
              disabled={index === tabs.length - 1}
              onclick={() => move(index, 1)}
            >
              ↓
            </button>
          </td>
          <th scope="row">
            <span class="flex items-center gap-2 font-normal">
              <Picture image={tab.image} name={tab.name} />
              {tab.name}
            </span>
          </th>
          <td>{categoryName(tab)}</td>
          <td>{KINDS[tab.type]}</td>
          <td>
            {#if tab.group_id === tab.group_id_max}
              {groupName(tab.group_id)}
            {:else}
              {groupName(tab.group_id_max)} to {groupName(tab.group_id)}
            {/if}
          </td>
          <td>
            <input
              type="checkbox"
              class="toggle toggle-sm"
              aria-label={`Show ${tab.name} in the menu`}
              checked={tab.enabled === 1}
              onchange={(event) =>
                run(
                  () =>
                    api('PUT', `tabs/${tab.id}`, {
                      enabled: asSwitch(event.currentTarget.checked),
                    }),
                  'The tab is changed.',
                )}
            />
          </td>
          <td>
            <input
              type="radio"
              class="radio radio-sm"
              name="default-tab"
              aria-label={`Open ${tab.name} at the start`}
              checked={tab.default === 1}
              onchange={() =>
                run(() => api('PUT', `tabs/${tab.id}`, { default: 1 }), 'The tab is changed.')}
            />
          </td>
          <td class="whitespace-nowrap text-right">
            <button class="btn btn-ghost btn-xs" onclick={() => edit(tab)}>
              Change<span class="sr-only"> {tab.name}</span>
            </button>
            <button class="btn btn-ghost btn-xs text-error" onclick={() => remove(tab)}>
              Delete<span class="sr-only"> {tab.name}</span>
            </button>
          </td>
        </tr>
      {:else}
        <tr><td colspan="8">There is no tab yet.</td></tr>
      {/each}
    </tbody>
  </table>
</div>

{#if dialog}
  {#key dialog.tab?.id}
    <TabDialog
      tab={dialog.tab}
      categories={inMenuOrder(data.categories)}
      groups={data.groups}
      onsave={save}
      onclose={() => (dialog = null)}
    />
  {/key}
{/if}
