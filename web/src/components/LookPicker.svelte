<script lang="ts">
  // The choice of the look of each person: the layout (OG Organizr or the modern one), and
  // for it the theme and, on the modern layout, its colours. It is plain markup, so the menu of the
  // user of the OG Organizr layout, the Account page and the popover of the modern top bar
  // can all show it. Each change is saved at once and the page then loads the look again.
  import { api, failureText } from '../lib/api';
  import { MODERN_THEMES, modernThemeId } from '../lib/look';
  import { session } from '../lib/session.svelte';
  import { NATIVE_THEMES, themeId, themeLabel } from '../lib/theme';
  import ThemeSwatch from './ThemeSwatch.svelte';

  const id = $props.id();

  let error = $state<string | null>(null);
  let busy = $state(false);
  // Changes when a save fails, so that the controls show the saved look again.
  let generation = $state(0);

  const look = $derived(session.look);
  const modern = $derived(look.layout === 'modern');
  const COLOUR_SETS = [
    { id: 'dark', label: 'Dark' },
    { id: 'light', label: 'Light' },
  ];

  // Each theme shows a sample of its colours: on modern in the colour set that a choice of
  // it would give (see `chooseTheme`), on OG in its one daisyUI theme.
  const themes = $derived(
    modern
      ? MODERN_THEMES.map((theme) => ({
          id: theme.id,
          label: theme.label,
          swatch: modernThemeId(theme.id, colourSetFor(theme.id)),
        }))
      : NATIVE_THEMES.map((name) => ({ id: name, label: themeLabel(name), swatch: themeId(name) })),
  );
  const colourSets = $derived(
    modern
      ? (MODERN_THEMES.find((theme) => theme.id === look.theme)?.colourSets ?? COLOUR_SETS)
      : COLOUR_SETS,
  );

  async function send(body: {
    layout: string | null;
    theme?: string | null;
    colourSet?: string | null;
  }) {
    busy = true;
    error = null;
    try {
      await api('PUT', 'user/look', body);
      await session.load();
    } catch (failure) {
      error = failureText(failure);
      generation += 1;
    } finally {
      busy = false;
    }
  }

  /** The layout changes; its theme and colours stay as the person last chose them there. */
  const chooseLayout = (layout: 'og' | 'modern') => {
    if (layout !== look.layout) {
      return send({ layout });
    }
  };

  /** The colour set of a theme after a choice of it: the current one if the theme has it. */
  function colourSetFor(theme: string): string {
    // A colour set that the new theme does not have gives way to the first one it has.
    const sets = modern
      ? (MODERN_THEMES.find((entry) => entry.id === theme)?.colourSets ?? COLOUR_SETS)
      : COLOUR_SETS;
    return sets.some((set) => set.id === look.colourSet)
      ? look.colourSet
      : (sets[0]?.id ?? look.colourSet);
  }

  const chooseTheme = (theme: string) =>
    send({ layout: look.layout, theme, colourSet: colourSetFor(theme) });

  const chooseColourSet = (colourSet: string) =>
    send({ layout: look.layout, theme: look.theme, colourSet });
</script>

<div class="flex flex-col gap-3">
  {#key generation}
    <fieldset class="flex flex-col gap-1">
      <legend class="mb-1 text-sm font-semibold">Layout</legend>
      {#each [{ id: 'og', label: 'OG Organizr' }, { id: 'modern', label: 'Modern' }] as const as layout (layout.id)}
        <label class="flex items-center gap-2">
          <input
            type="radio"
            class="radio radio-sm"
            name={`${id}-layout`}
            checked={look.layout === layout.id}
            disabled={busy}
            onchange={() => chooseLayout(layout.id)}
          />
          <span>{layout.label}</span>
        </label>
      {/each}
    </fieldset>

    <fieldset class="flex flex-col gap-1">
      <legend class="mb-1 text-sm font-semibold">Theme</legend>
      <!-- Two columns on a wide screen, one on a phone; a long list scrolls in the popover. -->
      <div
        class="grid max-h-[50vh] grid-cols-1 gap-x-3 gap-y-1 overflow-y-auto p-0.5 sm:grid-cols-2"
      >
        {#each themes as theme (theme.id)}
          <label class="flex items-center gap-2">
            <input
              type="radio"
              class="radio radio-sm"
              name={`${id}-theme`}
              checked={look.theme === theme.id}
              disabled={busy}
              onchange={() => chooseTheme(theme.id)}
            />
            <ThemeSwatch theme={theme.swatch} />
            <span>{theme.label}</span>
          </label>
        {/each}
      </div>
    </fieldset>

    {#if modern}
      <fieldset class="flex flex-col gap-1">
        <legend class="mb-1 text-sm font-semibold">Colours</legend>
        {#each colourSets as set (set.id)}
          <label class="flex items-center gap-2">
            <input
              type="radio"
              class="radio radio-sm"
              name={`${id}-colours`}
              checked={look.colourSet === set.id}
              disabled={busy || colourSets.length < 2}
              onchange={() => chooseColourSet(set.id)}
            />
            <ThemeSwatch theme={modernThemeId(look.theme, set.id)} />
            <span>{set.label}</span>
          </label>
        {/each}
      </fieldset>
    {/if}
  {/key}

  <div class="flex items-center gap-2">
    <button
      type="button"
      class="btn btn-sm"
      disabled={busy}
      onclick={() => send({ layout: null, theme: null, colourSet: null })}
    >
      Use the default
    </button>
    {#if look.source === 'default'}
      <span class="text-xs">This is the default of the install.</span>
    {/if}
  </div>

  {#if error}
    <div role="alert" class="alert alert-error"><span>{error}</span></div>
  {/if}
</div>
