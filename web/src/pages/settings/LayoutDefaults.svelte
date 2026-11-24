<script lang="ts">
  // The top of the group "Colors & Themes" (shown as "Style & Themes") of the Appearance
  // settings: the look that a person sees until they choose their own. First the default
  // layout, then the theme of that layout (on Modern also the OG theme, under its own
  // heading), then a heading over PHP's colours of the OG layout. The values live in a
  // `LookForm`, which the form saves with its one Save.
  // Only the Admin changes the layout and the modern theme (D-125); a Co-Admin changes
  // the theme of the OG layout, as in PHP's form. PHP's setting `style` has no effect (D-92),
  // so the page has no field for it.
  import ThemeSwatch from '../../components/ThemeSwatch.svelte';
  import { modernThemeId, MODERN_THEMES } from '../../lib/look';
  import type { LookForm } from '../../lib/lookForm.svelte';
  import { themeId } from '../../lib/theme';

  let {
    look,
  }: {
    /** The values of the look, which the form loads and saves. */
    look: LookForm;
  } = $props();

  const canChange = $derived(look.canChangeLayout());
  // The OG theme shows on both layouts: while the default is Modern it stands below the
  // modern theme, so that the Admin changes both without a switch of the layout.
  const showModern = $derived(look.form.layout === 'modern');
</script>

{#if look.loadError}
  <div role="alert" class="alert alert-error mb-6"><span>{look.loadError}</span></div>
{:else if look.start}
  <div class="org-form-grid mb-6">
    <p class="org-form-wide text-sm">
      This is the look that a person sees until they choose their own. The OG Organizr layout is the
      one of PHP Organizr; the modern layout has a top bar and a menu grouped by category.
    </p>
    <div class="org-form-group org-form-wide">
      <label class="org-form-label" for="layout-default">Default layout</label>
      <select
        id="layout-default"
        class="org-form-control"
        value={look.form.layout}
        disabled={!canChange}
        onchange={(event) => (look.form.layout = event.currentTarget.value)}
      >
        <option value="og">OG Organizr</option>
        <option value="modern">Modern</option>
      </select>
    </div>
    {#if showModern}
      <div class="org-form-group">
        <label class="org-form-label" for="layout-theme">Modern theme</label>
        <select
          id="layout-theme"
          class="org-form-control"
          value={look.form.modernTheme}
          disabled={!canChange}
          onchange={(event) => look.chooseTheme(event.currentTarget.value)}
        >
          {#each MODERN_THEMES as theme (theme.id)}
            <option value={theme.id}>{theme.label}</option>
          {/each}
        </select>
      </div>
      <div class="org-form-group">
        <label class="org-form-label" for="layout-colours">Modern colour set</label>
        <!-- A sample of the colours of the chosen theme and set. -->
        <div class="flex items-center gap-2">
          <select
            id="layout-colours"
            class="org-form-control"
            value={look.form.modernColourSet}
            disabled={!canChange || look.colourSets.length < 2}
            onchange={(event) => (look.form.modernColourSet = event.currentTarget.value)}
          >
            {#each look.colourSets as set (set.id)}
              <option value={set.id}>{set.label}</option>
            {/each}
          </select>
          <ThemeSwatch theme={modernThemeId(look.form.modernTheme, look.form.modernColourSet)} />
        </div>
      </div>
      <!-- The theme of the OG layout, for the people who choose that layout. -->
      <h3 class="org-form-wide mt-2 font-semibold">OG Organizr</h3>
    {/if}
    <div class="org-form-group">
      <label class="org-form-label" for="php-theme">Theme</label>
      <div class="flex items-center gap-2">
        <select
          id="php-theme"
          class="org-form-control"
          value={look.form.theme}
          onchange={(event) => (look.form.theme = event.currentTarget.value)}
        >
          {#each look.themes as option (String(option.value))}
            <option value={String(option.value)} disabled={option.disabled}>{option.name}</option>
          {/each}
        </select>
        <!-- A sample of the colours of the chosen theme. -->
        <ThemeSwatch theme={themeId(look.form.theme)} />
      </div>
    </div>
    <div class="org-form-wide mt-4">
      <h3 class="font-semibold">Colours of the OG Organizr layout</h3>
      <p class="text-sm">
        These colours apply to the OG Organizr layout only. A modern theme sets its own colours.
      </p>
    </div>
  </div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
