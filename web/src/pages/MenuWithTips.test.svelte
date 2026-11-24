<script lang="ts">
  // For the tests only: the menu inside the provider of tooltips that the modern shell
  // gives it, as the column of icons needs one.
  import type { ComponentProps } from 'svelte';

  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import Menu from './Menu.svelte';

  let props: ComponentProps<typeof Menu> = $props();
</script>

<Tooltip.Provider>
  <Menu {...props} />
</Tooltip.Provider>
