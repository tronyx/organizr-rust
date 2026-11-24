<script lang="ts">
  // Tab Editor, Homepage Items: the cards of the homepage items of PHP Organizr. A card
  // shows the picture, the name and the category; the picture opens the form of the item
  // in a dialog, as `editHomepageItem()` of PHP does. The cards are the items that the
  // license of the install has, as PHP lists them.
  import { onMount } from 'svelte';

  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import { HOMEPAGE_ITEMS } from '../../settings/forms';
  import type { HomepageItemForm } from '../../settings/model';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let items = $state<HomepageItemForm[] | null>(null);
  let error = $state<string | null>(null);
  let open = $state<HomepageItemForm | null>(null);

  /** Whether the license holds the item, as PHP's `strpos(text, license) !== false`. */
  function licensed(item: HomepageItemForm, license: string): boolean {
    return item.license === undefined || item.license.includes(license);
  }

  onMount(async () => {
    try {
      const config = (await api<Record<string, unknown>>('GET', 'config')).data;
      const license = String(config.license ?? '');
      items = HOMEPAGE_ITEMS.filter((item) => licensed(item, license));
    } catch (failure) {
      error = failureText(failure);
    }
  });
</script>

<section class="org-panel" aria-labelledby="homepage-items-title">
  <div class="org-panel-heading">
    <h2 id="homepage-items-title">Homepage Items</h2>
  </div>
  {#if error}
    <div role="alert" class="alert alert-error m-4"><span>{error}</span></div>
  {:else if items}
    <ul class="org-plugin-cards">
      {#each items as item (item.id)}
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
    {#key open.id}
      <SettingsForm page={open.page} {ondone} />
    {/key}
  </Dialog>
{/if}
