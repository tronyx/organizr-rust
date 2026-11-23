<script lang="ts">
  // The group "Layout" of the Appearance settings: the layout, theme and colour set that
  // a person sees until they choose their own. The Admin saves them; a Co-Admin sees them.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { MODERN_THEMES } from '../../lib/look';
  import { session } from '../../lib/session.svelte';

  let {
    canChange,
    ondone,
  }: {
    /** Whether the person may save: the Admin. */
    canChange: boolean;
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  interface Defaults {
    layout: string;
    modernTheme: string;
    modernColourSet: string;
  }

  let start = $state<Defaults | null>(null);
  let form = $state<Defaults>({ layout: 'og', modernTheme: 'organizr', modernColourSet: 'dark' });
  let loadError = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const colourSets = $derived(
    MODERN_THEMES.find((theme) => theme.id === form.modernTheme)?.colourSets ?? [],
  );
  const changed = $derived(
    start !== null &&
      (form.layout !== start.layout ||
        form.modernTheme !== start.modernTheme ||
        form.modernColourSet !== start.modernColourSet),
  );

  function text(value: unknown, fallback: string): string {
    return typeof value === 'string' && value !== '' ? value : fallback;
  }

  async function load() {
    try {
      const config = (await api<Record<string, unknown>>('GET', 'config')).data;
      const loaded = {
        layout: text(config.layout, 'og'),
        modernTheme: text(config.modernTheme, 'organizr'),
        modernColourSet: text(config.modernColourSet, 'dark'),
      };
      start = loaded;
      form = { ...loaded };
      loadError = null;
    } catch (failure) {
      loadError = failureText(failure);
    }
  }

  function chooseTheme(theme: string) {
    form.modernTheme = theme;
    const sets = MODERN_THEMES.find((entry) => entry.id === theme)?.colourSets ?? [];
    if (!sets.some((set) => set.id === form.modernColourSet) && sets[0]) {
      form.modernColourSet = sets[0].id;
    }
  }

  async function save() {
    busy = true;
    error = null;
    try {
      await api('PUT', 'config', { ...form });
      await Promise.all([load(), session.load()]);
      ondone('The settings are saved.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

{#if loadError}
  <div role="alert" class="alert alert-error"><span>{loadError}</span></div>
{:else if start}
  <div class="org-form-grid mb-6">
    <p class="org-form-wide text-sm">
      This is the look that a person sees until they choose their own. The OG Organizr layout is the
      one of PHP Organizr; the modern layout has a top bar and a menu grouped by category.
    </p>
    <div class="org-form-group">
      <label class="org-form-label" for="layout-default">Default layout</label>
      <select
        id="layout-default"
        class="org-form-control"
        value={form.layout}
        disabled={!canChange}
        onchange={(event) => (form.layout = event.currentTarget.value)}
      >
        <option value="og">OG Organizr</option>
        <option value="modern">Modern</option>
      </select>
    </div>
    <div class="org-form-group">
      <label class="org-form-label" for="layout-theme">Modern theme</label>
      <select
        id="layout-theme"
        class="org-form-control"
        value={form.modernTheme}
        disabled={!canChange}
        onchange={(event) => chooseTheme(event.currentTarget.value)}
      >
        {#each MODERN_THEMES as theme (theme.id)}
          <option value={theme.id}>{theme.label}</option>
        {/each}
      </select>
    </div>
    <div class="org-form-group">
      <label class="org-form-label" for="layout-colours">Modern colour set</label>
      <select
        id="layout-colours"
        class="org-form-control"
        value={form.modernColourSet}
        disabled={!canChange || colourSets.length < 2}
        onchange={(event) => (form.modernColourSet = event.currentTarget.value)}
      >
        {#each colourSets as set (set.id)}
          <option value={set.id}>{set.label}</option>
        {/each}
      </select>
    </div>
  </div>
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  {#if canChange}
    <button type="button" class="btn btn-info btn-sm" disabled={!changed || busy} onclick={save}>
      {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
      Save the layout
    </button>
  {/if}
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
