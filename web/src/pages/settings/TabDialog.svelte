<script lang="ts">
  // The form for a new tab, or for a tab that is changed.
  import Dialog from '../../components/Dialog.svelte';
  import PictureField from '../../components/PictureField.svelte';
  import { asSwitch } from '../../lib/editor';
  import type { Category, Group, Tab } from '../../lib/types';

  let {
    tab,
    categories,
    groups,
    onsave,
    onclose,
  }: {
    /** The tab to change, with its addresses as they are stored. `null` for a new tab. */
    tab: Tab | null;
    categories: Category[];
    groups: Group[];
    /** Stores the fields. Returns why the server refused them, or `null`. */
    onsave: (fields: Record<string, unknown>) => Promise<string | null>;
    onclose: () => void;
  } = $props();

  const fallbackCategory = $derived(
    categories.find((category) => category.default === 1) ?? categories[0],
  );
  const fallbackGroup = $derived(groups.find((group) => group.default === 1));

  // The form starts with the values of the tab. The dialog is made anew for each tab.
  // svelte-ignore state_referenced_locally
  let name = $state(tab?.name ?? '');
  // svelte-ignore state_referenced_locally
  let url = $state(tab?.url ?? '');
  // svelte-ignore state_referenced_locally
  let urlLocal = $state(tab?.url_local ?? '');
  // svelte-ignore state_referenced_locally
  let image = $state(tab?.image ?? '');
  // svelte-ignore state_referenced_locally
  let category = $state(tab?.category_id ?? fallbackCategory?.category_id ?? 0);
  // svelte-ignore state_referenced_locally
  let kind = $state(tab?.type ?? 1);
  // svelte-ignore state_referenced_locally
  let leastAccess = $state(tab?.group_id ?? fallbackGroup?.group_id ?? 0);
  // svelte-ignore state_referenced_locally
  let mostAccess = $state(tab?.group_id_max ?? 0);
  // svelte-ignore state_referenced_locally
  let enabled = $state(tab ? tab.enabled === 1 : true);
  // svelte-ignore state_referenced_locally
  let isDefault = $state(tab?.default === 1);
  // svelte-ignore state_referenced_locally
  let addToAdmin = $state(tab?.add_to_admin === 1);
  // svelte-ignore state_referenced_locally
  let preload = $state(tab?.preload === 1);
  // svelte-ignore state_referenced_locally
  let ping = $state(tab?.ping === 1);
  // svelte-ignore state_referenced_locally
  let pingUrl = $state(tab?.ping_url ?? '');

  let error = $state<string | null>(null);
  let busy = $state(false);

  // A page of Organizr itself, such as the settings, keeps its kind and its address.
  const isInternal = $derived(tab?.type === 0);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = await onsave({
      name,
      ...(isInternal ? {} : { url, url_local: urlLocal, type: kind }),
      image,
      category_id: category,
      group_id: leastAccess,
      group_id_max: mostAccess,
      enabled: asSwitch(enabled),
      default: asSwitch(isDefault),
      add_to_admin: asSwitch(addToAdmin),
      preload: asSwitch(preload),
      ...(isInternal ? {} : { ping: asSwitch(ping), ping_url: pingUrl }),
    });
    busy = false;
  }
</script>

<Dialog title={tab ? `Change the tab ${tab.name}` : 'Add a tab'} {onclose}>
  <form class="mt-2" onsubmit={save}>
    <fieldset class="fieldset">
      <label class="label" for="tab-name">Name</label>
      <input id="tab-name" class="input w-full" bind:value={name} required maxlength="50" />

      {#if !isInternal}
        <label class="label" for="tab-url">Address</label>
        <input
          id="tab-url"
          class="input w-full"
          bind:value={url}
          placeholder="https://plex.example.com/"
          aria-describedby="tab-url-hint"
        />
        <p id="tab-url-hint" class="label whitespace-normal">
          The address can hold placeholders such as {'{username}'} and {'{domain}'}.
        </p>

        <label class="label" for="tab-url-local">Address on the local network</label>
        <input
          id="tab-url-local"
          class="input w-full"
          bind:value={urlLocal}
          placeholder="http://192.168.1.10:32400/"
          aria-describedby="tab-url-local-hint"
        />
        <p id="tab-url-local-hint" class="label whitespace-normal">
          A person on the local network of the server opens this address. It can stay empty.
        </p>
      {/if}

      <label class="label" for="tab-image">Picture</label>
      <PictureField id="tab-image" bind:value={image} {name} />

      <div class="grid gap-x-4 sm:grid-cols-2">
        <div>
          <label class="label" for="tab-category">Category</label>
          <select id="tab-category" class="select w-full" bind:value={category}>
            {#each categories as option (option.id)}
              <option value={option.category_id}>{option.category}</option>
            {/each}
          </select>
        </div>
        {#if !isInternal}
          <div>
            <label class="label" for="tab-kind">Opens</label>
            <select id="tab-kind" class="select w-full" bind:value={kind}>
              <option value={1}>Inside Organizr</option>
              <option value={2}>In a new window</option>
            </select>
          </div>
        {/if}
        <div>
          <label class="label" for="tab-most-access">Shown to the groups from</label>
          <select id="tab-most-access" class="select w-full" bind:value={mostAccess}>
            {#each groups as group (group.id)}
              <option value={group.group_id}>{group.group}</option>
            {/each}
          </select>
        </div>
        <div>
          <label class="label" for="tab-least-access">down to</label>
          <select id="tab-least-access" class="select w-full" bind:value={leastAccess}>
            {#each groups as group (group.id)}
              <option value={group.group_id}>{group.group}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="mt-2 grid gap-2 sm:grid-cols-2">
        <label class="label">
          <input type="checkbox" class="toggle" bind:checked={enabled} />
          Shown in the menu
        </label>
        <label class="label">
          <input type="checkbox" class="toggle" bind:checked={isDefault} />
          Opens at the start
        </label>
        <label class="label">
          <input type="checkbox" class="toggle" bind:checked={addToAdmin} />
          Also shown to Admins and Co-Admins
        </label>
        {#if !isInternal}
          <label class="label">
            <input type="checkbox" class="toggle" bind:checked={preload} />
            Loaded at the start
          </label>
          <label class="label">
            <input type="checkbox" class="toggle" bind:checked={ping} />
            Checked whether it answers
          </label>
        {/if}
      </div>

      {#if ping && !isInternal}
        <label class="label" for="tab-ping-url">Address to check</label>
        <input
          id="tab-ping-url"
          class="input w-full"
          bind:value={pingUrl}
          required
          placeholder="plex.example.com:32400"
          aria-describedby="tab-ping-url-hint"
        />
        <p id="tab-ping-url-hint" class="label whitespace-normal">
          A host with a port is checked with a connection to that port. A host without a port is
          checked with a ping. The menu shows a dot for the answer.
        </p>
      {/if}
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
