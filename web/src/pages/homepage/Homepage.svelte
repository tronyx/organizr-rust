<script lang="ts">
  // The homepage: the items that the server lists for this person, in their order.
  import { onMount } from 'svelte';

  import Notice from '../../components/Notice.svelte';
  import { api, failureText } from '../../lib/api';
  import type { HomepageItem } from '../../lib/homepage';
  import CustomHtml from './CustomHtml.svelte';

  let { title }: { title: string } = $props();

  /** The kinds of items that this interface can show. */
  const KNOWN = new Set(['customhtml']);

  let items = $state<HomepageItem[] | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const listed = (await api<HomepageItem[]>('GET', 'homepage/items')).data;
      items = listed.filter((item) => KNOWN.has(item.kind));
    } catch (failure) {
      error = failureText(failure);
    }
  });
</script>

{#if error}
  <Notice {title}>
    <div role="alert" class="alert alert-error"><span>{error}</span></div>
  </Notice>
{:else if items === null}
  <main class="grid min-h-[60vh] place-items-center" aria-busy="true">
    <span class="loading loading-spinner loading-lg" aria-label="The homepage is loading"></span>
  </main>
{:else if items.length === 0}
  <Notice {title}>
    <p>The homepage has no items for you.</p>
  </Notice>
{:else}
  <main class="flex flex-col gap-4 p-4" aria-label={title}>
    {#each items as item (item.key)}
      <section data-item={item.key}>
        {#if item.kind === 'customhtml'}
          <CustomHtml id={item.id} />
        {/if}
      </section>
    {/each}
  </main>
{/if}
