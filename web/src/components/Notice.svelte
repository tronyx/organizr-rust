<script lang="ts">
  // A short text in the middle of the main area, for a page that has nothing else to show.
  import type { Snippet } from 'svelte';

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<main class="grid min-h-[60vh] place-items-center p-4">
  <div class="card bg-base-200 w-full max-w-md shadow">
    <div class="card-body">
      <h1 class="card-title">{title}</h1>
      {@render children()}
    </div>
  </div>
</main>
