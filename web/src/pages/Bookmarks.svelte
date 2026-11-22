<script lang="ts">
  // The page of the bookmark plugin: links as tiles, in their categories.
  import { onMount } from 'svelte';

  import Notice from '../components/Notice.svelte';
  import BookmarkTiles, { type BookmarkCategory } from '../components/BookmarkTiles.svelte';
  import { api, failureText } from '../lib/api';

  interface Answer {
    categories: BookmarkCategory[];
  }

  let { title }: { title: string } = $props();

  let categories = $state<Answer['categories'] | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      categories = (await api<Answer>('GET', 'plugins/bookmark/page')).data.categories;
    } catch (failure) {
      error = failureText(failure);
    }
  });
</script>

{#if error}
  <Notice {title}>
    <div role="alert" class="alert alert-error"><span>{error}</span></div>
  </Notice>
{:else if categories === null}
  <main class="grid min-h-[60vh] place-items-center" aria-busy="true">
    <span class="loading loading-spinner loading-lg" aria-label="The bookmarks are loading"></span>
  </main>
{:else if categories.length === 0}
  <Notice {title}>
    <p>There are no bookmarks for you.</p>
  </Notice>
{:else}
  <main class="flex flex-col gap-6 p-4" aria-label={title}>
    <BookmarkTiles {categories} />
  </main>
{/if}
