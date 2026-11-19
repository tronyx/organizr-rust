<script lang="ts">
  // The settings: the editors for tabs and for categories.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { PAGES } from '../../lib/route.svelte';
  import { session } from '../../lib/session.svelte';
  import type { EditorData } from '../../lib/types';
  import CategoriesEditor from './CategoriesEditor.svelte';
  import TabsEditor from './TabsEditor.svelte';

  let { section }: { section: 'tabs' | 'categories' } = $props();

  let data = $state<EditorData | null>(null);
  let loadError = $state<string | null>(null);
  // What the last change did, for a person who cannot see the table change.
  let done = $state<string | null>(null);

  async function load() {
    try {
      data = (await api<EditorData>('GET', 'tabs')).data;
      loadError = null;
    } catch (failure) {
      loadError = failureText(failure);
    }
  }

  /**
   * Runs a change on the server. Afterwards the editors and the menu load their data
   * again, also after a failure: the server may have carried a part of it out.
   */
  async function change(request: () => Promise<unknown>, success: string): Promise<string | null> {
    let failure: string | null = null;
    try {
      await request();
      done = success;
    } catch (caught) {
      failure = failureText(caught);
      done = null;
    }
    await Promise.all([load(), session.load()]);
    return failure;
  }

  onMount(load);
</script>

<main class="mx-auto max-w-6xl p-4">
  <h1 class="text-2xl font-bold">Settings</h1>

  <div role="tablist" class="tabs tabs-border mt-2 mb-4">
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'tabs'}
      aria-selected={section === 'tabs'}
      href={PAGES.tabSettings}
    >
      Tabs
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'categories'}
      aria-selected={section === 'categories'}
      href={PAGES.categorySettings}
    >
      Categories
    </a>
  </div>

  <p role="status" class="text-success mb-2 min-h-6 text-sm">{done ?? ''}</p>

  {#if data}
    {#if section === 'tabs'}
      <TabsEditor {data} {change} />
    {:else}
      <CategoriesEditor {data} {change} />
    {/if}
  {:else if loadError}
    <div role="alert" class="alert alert-error">
      <span>{loadError}</span>
    </div>
  {:else}
    <span class="loading loading-spinner" aria-label="Loading"></span>
  {/if}
</main>
