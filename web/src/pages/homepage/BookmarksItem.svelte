<script lang="ts">
  // The Bookmarks item of the homepage: the bookmarks of the bookmark plugin, as its page
  // shows them, loaded again after the time of `homepageBookmarksRefresh`. PHP Organizr
  // puts the HTML of the page of the plugin here; this application draws the same tiles.
  import BookmarkTiles, { type BookmarkCategory } from '../../components/BookmarkTiles.svelte';
  import { itemData } from '../../lib/itemData.svelte';

  const item = itemData<{ categories: BookmarkCategory[] }>(
    'plugins/bookmark/page',
    'homepageBookmarksRefresh',
  );
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Bookmarks: {item.error}</span></div>
{:else if item.data}
  <div class="flex flex-col gap-6" data-testid="bookmarks-item">
    <BookmarkTiles categories={item.data.categories} />
  </div>
{/if}
