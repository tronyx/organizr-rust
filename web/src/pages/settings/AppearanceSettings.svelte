<script lang="ts">
  // Customize > Appearance: the theme, the colours that stand on top of it, and the style
  // sheets of the Admin. The themes are those that the install has: the built-in ones
  // and the installed ones of the marketplace that this interface has rebuilt.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { appearanceGroups } from '../../lib/settingsForm';
  import { isNativeTheme, themeLabel } from '../../lib/theme';
  import { PAGES } from '../../lib/route.svelte';
  import SettingsForm from './SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let themes = $state<{ value: string; label: string }[] | null>(null);
  let loadError = $state<string | null>(null);

  onMount(async () => {
    try {
      const { data } = await api<Record<string, unknown>>('GET', 'themes');
      themes = Object.keys(data ?? {})
        .filter(isNativeTheme)
        .map((name) => ({ value: name, label: themeLabel(name) }));
    } catch (failure) {
      loadError = failureText(failure);
    }
  });
</script>

{#if themes}
  <p class="mb-4 max-w-2xl text-sm">
    More themes are in the <a class="link" href={PAGES.themeSettings}>theme marketplace</a>.
  </p>
  <SettingsForm groups={appearanceGroups(themes)} {ondone} />
{:else if loadError}
  <div role="alert" class="alert alert-error">
    <span>{loadError}</span>
  </div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
