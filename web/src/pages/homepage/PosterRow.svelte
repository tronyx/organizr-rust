<script lang="ts">
  // A row of posters as PHP Organizr shows it: a thin title, two buttons that move the
  // row, and the posters side by side without a scroll bar.
  import type { Snippet } from 'svelte';

  let {
    title,
    label,
    controls,
    children,
  }: {
    /** The title above the row. */
    title: string;
    /** The name of the list for assistive tools. */
    label: string;
    /** More controls at the right of the title. */
    controls?: Snippet;
    /** The posters: `li` elements. */
    children: Snippet;
  } = $props();

  let row = $state<HTMLUListElement>();

  /** Moves the row by most of its width. */
  function move(direction: 1 | -1) {
    row?.scrollBy({ left: direction * row.clientWidth * 0.8, behavior: 'smooth' });
  }
</script>

<div class="flex flex-wrap items-center gap-x-4">
  <h2 class="org-item-title org-plain">{title}<span aria-hidden="true">:</span></h2>
  <div class="ml-auto flex items-center gap-3">
    {@render controls?.()}
    <div class="org-row-buttons">
      <button type="button" aria-label="Earlier in {label}" onclick={() => move(-1)}>
        <i class="oi oi-fontawesome fa-chevron-left" aria-hidden="true"></i>
      </button>
      <button type="button" aria-label="Later in {label}" onclick={() => move(1)}>
        <i class="oi oi-fontawesome fa-chevron-right" aria-hidden="true"></i>
      </button>
    </div>
  </div>
</div>
<ul class="org-posters" aria-label={label} bind:this={row}>
  {@render children()}
</ul>
