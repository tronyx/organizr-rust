<script lang="ts">
  // The menu of the settings for the modern layout: the groups of PHP Organizr's main
  // tabs, each with its pages as links. Below 1024 pixels it is a button that opens the
  // same list in a sheet. Only the modern layout loads this file.
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { MediaQuery } from 'svelte/reactivity';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import { PAGES } from '../../lib/route.svelte';
  import { SETTINGS_MENU, settingsPlace } from '../../lib/settingsMenu';

  let { section }: { section: string } = $props();

  // The same width as the modern shell: below it the menu is a drawer.
  const wide = new MediaQuery('min-width: 1024px');
  let open = $state(false);

  const place = $derived(settingsPlace(section));

  $effect(() => {
    if (wide.current) {
      open = false;
    }
  });
</script>

{#snippet list(onpick?: () => void)}
  <nav aria-label="Settings" class="m-settings-nav">
    {#each SETTINGS_MENU as group, index (group.name)}
      <div class="m-settings-group">
        <!-- Not a heading: the page heading of the settings comes first in the page. -->
        <div class="m-settings-group-name" id="m-settings-group-{index}">
          <i class="oi-themify {group.icon}" aria-hidden="true"></i>
          <span>{group.name}</span>
        </div>
        <ul aria-labelledby="m-settings-group-{index}">
          {#each group.pages as entry (entry.section)}
            <li>
              <a
                href={PAGES[entry.page]}
                aria-current={entry.section === section ? 'page' : undefined}
                onclick={onpick}
              >
                {entry.label}
              </a>
            </li>
          {/each}
        </ul>
      </div>
    {/each}
  </nav>
{/snippet}

{#if wide.current}
  {@render list()}
{:else}
  <Sheet.Root bind:open>
    <Sheet.Trigger>
      {#snippet child({ props })}
        <Button variant="outline" class="m-settings-button" {...props}>
          <span class="m-settings-button-text">
            <span class="m-settings-button-label">Settings menu</span>
            <span class="m-settings-button-place">{place.group.name} · {place.page.label}</span>
          </span>
          <ChevronDownIcon aria-hidden="true" />
        </Button>
      {/snippet}
    </Sheet.Trigger>
    <Sheet.Content side="left" showCloseButton={false} class="m-settings-sheet p-0">
      <Sheet.Title class="m-settings-sheet-head">Settings</Sheet.Title>
      <Sheet.Description class="sr-only">The pages of the settings, by group.</Sheet.Description>
      {@render list(() => (open = false))}
    </Sheet.Content>
  </Sheet.Root>
{/if}
