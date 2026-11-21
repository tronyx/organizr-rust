<script lang="ts">
  // A row of posters as PHP Organizr shows it: a thin title, two buttons that move the
  // row and, for some rows, a button with a menu of filters, and the posters side by side
  // without a scroll bar.
  import type { Snippet } from 'svelte';

  let {
    title,
    label,
    filters,
    buttons,
    children,
  }: {
    /** The title above the row. */
    title: string;
    /** The name of the list for assistive tools. */
    label: string;
    /** The filters of the row, in the menu of the filter button. */
    filters?: Snippet;
    /** More buttons at the end of the group, such as the search of the requests. */
    buttons?: Snippet;
    /** The posters: `li` elements. */
    children: Snippet;
  } = $props();

  let row = $state<HTMLUListElement>();
  let filtersOpen = $state(false);

  /** Moves the row by most of its width. */
  function move(direction: 1 | -1) {
    row?.scrollBy({ left: direction * row.clientWidth * 0.8, behavior: 'smooth' });
  }
</script>

<div class="flex flex-wrap items-center gap-x-4">
  <h2 class="org-item-title org-plain">{title}<span aria-hidden="true">:</span></h2>
  <div class="relative ml-auto">
    <div class="org-row-buttons">
      <button type="button" aria-label="Earlier in {label}" onclick={() => move(-1)}>
        <i class="oi oi-fontawesome fa-chevron-left" aria-hidden="true"></i>
      </button>
      <button type="button" aria-label="Later in {label}" onclick={() => move(1)}>
        <i class="oi oi-fontawesome fa-chevron-right" aria-hidden="true"></i>
      </button>
      {#if filters}
        <button
          type="button"
          aria-label="Filter {label}"
          aria-expanded={filtersOpen}
          onclick={() => (filtersOpen = !filtersOpen)}
        >
          <i class="oi oi-fontawesome fa-filter" aria-hidden="true"></i>
          <span class="org-caret" aria-hidden="true"></span>
        </button>
      {/if}
      {@render buttons?.()}
    </div>
    {#if filters && filtersOpen}
      <div class="org-row-menu">
        {@render filters()}
      </div>
    {/if}
  </div>
</div>
<ul translate="no" class="org-posters" aria-label={label} bind:this={row}>
  {@render children()}
</ul>
