<script lang="ts">
  // Tab Editor, Homepage Items: the cards of the homepage items of PHP Organizr. A card
  // shows the picture, the name and the category; the picture opens the form of the item
  // in a dialog, as `editHomepageItem()` of PHP does.
  import { onMount } from 'svelte';

  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import type { PhpGroups } from '../../lib/phpForm';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  /** A homepage item, as `GET settings/homepage` lists it. */
  interface Item {
    name: string;
    /** Whether the licence of the install has the item. */
    enabled: boolean;
    image: string;
    category: string;
  }

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let items = $state<Item[] | null>(null);
  let error = $state<string | null>(null);
  let open = $state<Item | null>(null);

  onMount(async () => {
    try {
      items = (await api<Item[]>('GET', 'settings/homepage')).data.filter((item) => item.enabled);
    } catch (failure) {
      error = failureText(failure);
    }
  });
</script>

<section class="org-panel panel panel-default" aria-labelledby="homepage-items-title">
  <div class="org-panel-heading panel-heading">
    <h2 id="homepage-items-title">Homepage Items</h2>
  </div>
  {#if error}
    <div role="alert" class="alert alert-error m-4"><span>{error}</span></div>
  {:else if items}
    <ul class="org-plugin-cards">
      {#each items as item (item.name)}
        <li class="org-plugin-card">
          <button
            type="button"
            class="org-plugin-picture w-full cursor-pointer"
            aria-label={`Settings of ${item.name}`}
            onclick={() => (open = item)}
          >
            <img src={item.image} alt="" />
          </button>
          <h3>{item.name}</h3>
          <small>{item.category}</small>
        </li>
      {/each}
    </ul>
  {:else}
    <span class="loading loading-spinner m-6" aria-label="Loading"></span>
  {/if}
</section>

{#if open}
  <Dialog title={open.name} onclose={() => (open = null)}>
    <PhpSettingsForm
      path={`settings/homepage/${encodeURIComponent(open.name)}`}
      title={open.name}
      pick={(data) => (data as { settings: PhpGroups }).settings}
      {ondone}
    />
  </Dialog>
{/if}
