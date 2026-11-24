<script lang="ts">
  // The pager under a long table, as the pager of PHP Organizr: First, Prev, at most five
  // page numbers, Next and Last. The current page has the purple of PHP's `btn-primary`,
  // which is the secondary color of the OG theme.
  import { pageNumbers } from '../lib/paging';

  let {
    page,
    count,
    label,
    onpage,
  }: {
    /** The page that is shown, counted from 1. */
    page: number;
    /** How many pages there are. */
    count: number;
    /** What the pages are of, for the name of the pager: "Pages of the users". */
    label: string;
    onpage: (page: number) => void;
  } = $props();
</script>

<nav class="org-pager" aria-label={label}>
  <button
    class="btn btn-info"
    aria-label="First page"
    disabled={page <= 1}
    onclick={() => onpage(1)}
  >
    First
  </button>
  <button
    class="btn btn-info"
    aria-label="Previous page"
    disabled={page <= 1}
    onclick={() => onpage(page - 1)}
  >
    Prev
  </button>
  {#each pageNumbers(page, count) as number (number)}
    <button
      class="btn {number === page ? 'btn-secondary' : 'btn-info'}"
      aria-label="Page {number}"
      aria-current={number === page ? 'page' : undefined}
      onclick={() => onpage(number)}
    >
      {number}
    </button>
  {/each}
  <button
    class="btn btn-info"
    aria-label="Next page"
    disabled={page >= count}
    onclick={() => onpage(page + 1)}
  >
    Next
  </button>
  <button
    class="btn btn-info"
    aria-label="Last page"
    disabled={page >= count}
    onclick={() => onpage(count)}
  >
    Last
  </button>
</nav>
