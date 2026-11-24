<script lang="ts">
  // An entry of the modern menu with its name as a tooltip at the right, while the menu
  // is a column of icons. Otherwise the entry shows as it is, without a tooltip. The
  // entry keeps its own accessible name: the tooltip only shows the name to the eye, and
  // a screen reader does not hear the name a second time as a description.
  import type { Snippet } from 'svelte';

  import * as Tooltip from '$lib/components/ui/tooltip/index.js';

  let {
    label,
    on,
    children,
  }: {
    /** The name that the tooltip shows. */
    label: string;
    /** Whether the menu is a column of icons. */
    on: boolean;
    /** The entry. It takes the props of the trigger of the tooltip, to spread on itself. */
    children: Snippet<[Record<string, unknown>]>;
  } = $props();
</script>

{#if on}
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <!-- The entry has its name already, so it is not described by the same text. -->
        {@render children({ ...props, 'aria-describedby': undefined })}
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="right" translate="no">{label}</Tooltip.Content>
  </Tooltip.Root>
{:else}
  {@render children({})}
{/if}
