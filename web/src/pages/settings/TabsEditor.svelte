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

  /** The kinds of tab, with the names that the Tab Editor of PHP gives them. */
  const KINDS = ['Organizr', 'iFrame', 'New Window'] as const;

  /** The switches of a tab that the table changes at once, with the column of PHP. */
  const SWITCHES = [
    ['enabled', 'Active'],
    ['splash', 'Splash'],
    ['ping', 'Ping'],
    ['preload', 'Preload'],
    ['add_to_admin', 'Add to Admin'],
  ] as const;

  const tabs = $derived(inMenuOrder(data.tabs));

  // The dialog: closed, open for a new tab, or open for a tab that is changed.
  let dialog = $state<{ tab: Tab | null } | null>(null);
  let error = $state<string | null>(null);

  const categories = $derived(inMenuOrder(data.categories));

  /** Changes one field of a tab at once, as the Tab Editor of PHP does. */
  function set(tab: Tab, fields: Record<string, unknown>) {
    void run(() => api('PUT', `tabs/${tab.id}`, fields), 'The tab is changed.');
  }

  /** The label of a switch for a person who cannot see the column. */
  function switchLabel(key: string, name: string): string {
    switch (key) {
      case 'enabled':
        return `Show ${name} in the menu`;
      case 'splash':
        return `Show ${name} on the splash page`;
      case 'ping':
        return `Ping ${name}`;
      case 'preload':
        return `Preload ${name}`;
      default:
        return `Add ${name} to the menu of the Admin`;
    }
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

{#if error}
  <div role="alert" class="alert alert-error mb-4">
    <span>{error}</span>
  </div>
{/if}

<section class="org-panel" aria-labelledby="tab-editor-title">
  <div class="org-panel-heading">
    <h2 id="tab-editor-title">Tab Editor</h2>
    <button
      class="org-round-button org-round-info org-round-small"
      aria-label="Add a tab"
      title="Add a tab"
      onclick={() => (dialog = { tab: null })}
    >
      <i class="oi-themify ti-plus" aria-hidden="true"></i>
    </button>
  </div>
  <div class="overflow-x-auto">
    <table class="org-table">
      <thead>
        <tr>
          <th scope="col"><span class="sr-only">Order</span></th>
          <th scope="col"><span class="sr-only">Picture</span></th>
          <th scope="col">Name</th>
          <th scope="col">Category</th>
          <th scope="col">Min Group</th>
          <th scope="col">Max Group</th>
          <th scope="col">Type</th>
          <th scope="col" class="text-center">Default</th>
          {#each SWITCHES as [key, label] (key)}
            <th scope="col" class="text-center">{label}</th>
          {/each}
          <th scope="col" class="text-center">Edit</th>
          <th scope="col" class="text-center">Delete</th>
        </tr>
      </thead>
      <tbody>
        {#each tabs as tab, index (tab.id)}
          <tr>
            <td class="whitespace-nowrap">
              <span class="org-order">
                <button
                  aria-label={`Move ${tab.name} up`}
                  disabled={index === 0}
                  onclick={() => move(index, -1)}
                >
                  <i class="oi-themify ti-angle-up" aria-hidden="true"></i>
                </button>
                <button
                  aria-label={`Move ${tab.name} down`}
                  disabled={index === tabs.length - 1}
                  onclick={() => move(index, 1)}
                >
                  <i class="oi-themify ti-angle-down" aria-hidden="true"></i>
                </button>
              </span>
            </td>
            <td class="org-table-picture"><Picture image={tab.image} name={tab.name} /></td>
            <th scope="row" translate="no">{tab.name}</th>
            <td>
              <select
                class="org-table-select"
                aria-label={`Category of ${tab.name}`}
                value={tab.category_id ?? 0}
                onchange={(event) => set(tab, { category_id: Number(event.currentTarget.value) })}
              >
                {#each categories as category (category.id)}
                  <option value={category.category_id} translate="no">{category.category}</option>
                {/each}
              </select>
            </td>
            <td>
              <select
                class="org-table-select"
                aria-label={`Least group that sees ${tab.name}`}
                value={tab.group_id}
                onchange={(event) => set(tab, { group_id: Number(event.currentTarget.value) })}
              >
                {#each data.groups as group (group.id)}
                  <option value={group.group_id} translate="no">{group.group}</option>
                {/each}
              </select>
            </td>
            <td>
              <select
                class="org-table-select"
                aria-label={`Most group that sees ${tab.name}`}
                value={tab.group_id_max}
                onchange={(event) => set(tab, { group_id_max: Number(event.currentTarget.value) })}
              >
                {#each data.groups as group (group.id)}
                  <option value={group.group_id} translate="no">{group.group}</option>
                {/each}
              </select>
            </td>
            <td>
              <select
                class="org-table-select"
                aria-label={`Type of ${tab.name}`}
                value={tab.type}
                onchange={(event) => set(tab, { type: Number(event.currentTarget.value) })}
              >
                {#each KINDS as kind, value (kind)}
                  <option {value}>{kind}</option>
                {/each}
              </select>
            </td>
            <td class="text-center">
              <input
                type="radio"
                class="org-table-check"
                name="default-tab"
                aria-label={`Open ${tab.name} at the start`}
                checked={tab.default === 1}
                onchange={() => set(tab, { default: 1 })}
              />
            </td>
            {#each SWITCHES as [key] (key)}
              <td class="text-center">
                <input
                  type="checkbox"
                  class="org-table-check"
                  aria-label={switchLabel(key, tab.name)}
                  checked={tab[key] === 1}
                  onchange={(event) => set(tab, { [key]: asSwitch(event.currentTarget.checked) })}
                />
              </td>
            {/each}
            <td class="text-center">
              <button class="org-round-button org-round-info" onclick={() => edit(tab)}>
                <i class="oi-themify ti-pencil-alt" aria-hidden="true"></i>
                <span class="sr-only">Change {tab.name}</span>
              </button>
            </td>
            <td class="text-center">
              <button class="org-round-button org-round-danger" onclick={() => remove(tab)}>
                <i class="oi-themify ti-trash" aria-hidden="true"></i>
                <span class="sr-only">Delete {tab.name}</span>
              </button>
            </td>
          </tr>
        {:else}
          <tr><td colspan="15">There is no tab yet.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>

{#if dialog}
  {#key dialog.tab?.id}
    <TabDialog
      tab={dialog.tab}
      {categories}
      groups={data.groups}
      onsave={save}
      onclose={() => (dialog = null)}
    />
  {/key}
{/if}
