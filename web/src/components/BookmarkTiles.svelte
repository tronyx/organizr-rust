<script lang="ts" module>
  export interface BookmarkTile {
    name: string;
    url: string;
    image: string;
    text_color: string;
    border_color: string;
    dark_color: string;
    light_color: string;
  }

  export interface BookmarkCategory {
    category: string;
    tabs: BookmarkTile[];
  }
</script>

<script lang="ts">
  // The bookmarks of the bookmark plugin as colored tiles, in their categories. The page
  // of the plugin and the Bookmarks item of the homepage show them.
  import Picture from './Picture.svelte';

  let { categories }: { categories: BookmarkCategory[] } = $props();

  /** The address of a bookmark, when it is one that a browser opens as a link. */
  function linkOf(tile: BookmarkTile): string | undefined {
    return /^(https?:)?\/\//.test(tile.url) || tile.url.startsWith('/') ? tile.url : undefined;
  }
</script>

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
