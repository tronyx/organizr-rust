<script lang="ts">
  // The page of the bookmark plugin: links as tiles, in their categories.
  import { onMount } from 'svelte';

  import Notice from '../components/Notice.svelte';
  import Picture from '../components/Picture.svelte';
  import { api, failureText } from '../lib/api';

  interface Tile {
    name: string;
    url: string;
    image: string;
    text_color: string;
    border_color: string;
    dark_color: string;
    light_color: string;
  }

  interface Answer {
    categories: { category: string; tabs: Tile[] }[];
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

  /** The address of a bookmark, when it is one that a browser opens as a link. */
  function linkOf(tile: Tile): string | undefined {
    return /^(https?:)?\/\//.test(tile.url) || tile.url.startsWith('/') ? tile.url : undefined;
  }
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
    {#each categories as category (category.category)}
      <section aria-label={category.category}>
        <h2 class="mb-2 text-lg font-semibold">{category.category}</h2>
        <ul class="grid gap-2 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-6">
          {#each category.tabs as tile (tile.name)}
            <li>
              <a
                class="flex items-center gap-2 rounded border p-2 no-underline hover:brightness-110"
                href={linkOf(tile)}
                target="_blank"
                rel="noopener noreferrer"
                style:border-color={tile.border_color}
                style:background="linear-gradient(90deg, {tile.dark_color} 0%, {tile.light_color} 70%,
                {tile.dark_color} 100%)"
                style:color={tile.text_color}
              >
                <Picture image={tile.image} name={tile.name} />
                <span class="truncate font-medium">{tile.name}</span>
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </main>
{/if}
