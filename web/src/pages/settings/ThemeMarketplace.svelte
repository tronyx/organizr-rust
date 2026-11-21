<script lang="ts">
  // Marketplace > Themes: the themes of the marketplace of PHP Organizr, with a button to
  // install each one and a button to remove it. This interface has rebuilt them all, so an
  // install downloads nothing: the theme is then offered in Customize > Appearance.
  import { onMount } from 'svelte';

  import ThemePreview from '../../components/ThemePreview.svelte';
  import { api, failureText } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { themeLabel } from '../../lib/theme';

  /** A theme as `GET /themes/marketplace` lists it. Only the fields that the page shows. */
  interface MarketTheme {
    author?: string;
    category?: string;
    description?: string;
    version?: string;
    website?: string;
    installed: boolean;
    status: string;
  }

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let themes = $state<[string, MarketTheme][] | null>(null);
  let loadError = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state<string | null>(null);

  async function load() {
    try {
      const { data } = await api<Record<string, MarketTheme>>('GET', 'themes/marketplace');
      themes = Object.entries(data ?? {});
      loadError = null;
    } catch (failure) {
      loadError = failureText(failure);
    }
  }

  /** Installs or removes a theme. The path names it in lower case, as PHP Organizr does. */
  async function manage(name: string, method: 'POST' | 'DELETE') {
    busy = name;
    error = null;
    try {
      await api(method, `themes/manage/${encodeURIComponent(name.toLowerCase())}`);
      ondone(
        method === 'POST'
          ? `The theme ${themeLabel(name)} is installed.`
          : `The theme ${themeLabel(name)} is removed.`,
      );
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = null;
      // The page falls back to the default theme when its theme is removed.
      await Promise.all([load(), session.load()]);
    }
  }

  onMount(load);
</script>

{#if themes}
  <p class="mb-4 max-w-3xl text-sm">
    These are the themes of the marketplace of PHP Organizr, rebuilt for this interface. An install
    downloads nothing: the theme is then offered under Appearance.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mb-4">
      <span>{error}</span>
    </div>
  {/if}
  <div class="overflow-x-auto">
    <table class="table">
      <thead>
        <tr>
          <th>Preview</th>
          <th>Theme</th>
          <th>Category</th>
          <th>Status</th>
          <th><span class="sr-only">Actions</span></th>
        </tr>
      </thead>
      <tbody>
        {#each themes as [name, theme] (name)}
          <tr>
            <td class="w-48"><ThemePreview theme={name} /></td>
            <td>
              <div class="font-medium">{themeLabel(name)}</div>
              <div class="text-sm opacity-70">{theme.version ?? ''}</div>
              <div class="text-sm opacity-70">{theme.author ?? ''}</div>
              {#if theme.description}
                <div class="text-sm">{theme.description}</div>
              {/if}
            </td>
            <td>{theme.category ?? ''}</td>
            <td>{theme.status}</td>
            <td>
              <div class="flex gap-2">
                <button
                  type="button"
                  class="btn btn-sm btn-info"
                  disabled={busy !== null || theme.status === 'Up to date'}
                  aria-label={`Install ${themeLabel(name)}`}
                  onclick={() => manage(name, 'POST')}
                >
                  {#if busy === name}<span class="loading loading-spinner loading-xs"></span>{/if}
                  {theme.status === 'Update Available' ? 'Update' : 'Install'}
                </button>
                <button
                  type="button"
                  class="btn btn-sm btn-error"
                  disabled={busy !== null || !theme.installed}
                  aria-label={`Remove ${themeLabel(name)}`}
                  onclick={() => manage(name, 'DELETE')}
                >
                  Remove
                </button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else if loadError}
  <div role="alert" class="alert alert-error">
    <span>{loadError}</span>
  </div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
