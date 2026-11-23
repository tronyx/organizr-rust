<script lang="ts">
  // The choice of the look of each person: the layout (the original one or the modern
  // one), and for it the theme and its colours. It is plain markup, so the menu of the
  // user of the original layout, the Account page and the popover of the modern top bar
  // can all show it. Each change is saved at once and the page then loads the look again.
  import { api, failureText } from '../lib/api';
  import { MODERN_THEMES } from '../lib/look';
  import { session } from '../lib/session.svelte';
  import { NATIVE_THEMES, themeLabel } from '../lib/theme';

  const id = $props.id();

  let error = $state<string | null>(null);
  let busy = $state(false);

  const look = $derived(session.look);
  const modern = $derived(look.layout === 'modern');
  const COLOUR_SETS = [
    { id: 'dark', label: 'Dark' },
    { id: 'light', label: 'Light' },
  ];

  const themes = $derived(
    modern
      ? MODERN_THEMES.map((theme) => ({ id: theme.id, label: theme.label }))
      : NATIVE_THEMES.map((name) => ({ id: name, label: themeLabel(name) })),
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

  function chooseTheme(theme: string) {
    // A colour set that the new theme does not have gives way to the first one it has.
    const sets = modern
      ? (MODERN_THEMES.find((entry) => entry.id === theme)?.colourSets ?? COLOUR_SETS)
      : COLOUR_SETS;
    const colourSet = sets.some((set) => set.id === look.colourSet)
      ? look.colourSet
      : (sets[0]?.id ?? look.colourSet);
    return send({ layout: look.layout, theme, colourSet });
  }

  const chooseColourSet = (colourSet: string) =>
    send({ layout: look.layout, theme: look.theme, colourSet });
</script>

<div class="flex flex-col gap-3">
  <fieldset class="flex flex-col gap-1">
    <legend class="mb-1 text-sm font-semibold">Layout</legend>
    {#each [{ id: 'og', label: 'Original' }, { id: 'modern', label: 'Modern' }] as const as layout (layout.id)}
      <label class="flex items-center gap-2">
        <input
          type="radio"
          class="radio radio-sm"
          name={`${id}-layout`}
          checked={look.layout === layout.id}
          onchange={() => chooseLayout(layout.id)}
        />
        <span>{layout.label}</span>
      </label>
    {/each}
  </fieldset>

  <div class="flex flex-col gap-1">
    <label class="text-sm font-semibold" for={`${id}-theme`}>Theme</label>
    <select
      id={`${id}-theme`}
      class="select select-sm w-full"
      value={look.theme}
      onchange={(event) => chooseTheme(event.currentTarget.value)}
    >
      {#each themes as theme (theme.id)}
        <option value={theme.id}>{theme.label}</option>
      {/each}
    </select>
  </div>

  <fieldset class="flex flex-col gap-1">
    <legend class="mb-1 text-sm font-semibold">Colours</legend>
    {#each colourSets as set (set.id)}
      <label class="flex items-center gap-2">
        <input
          type="radio"
          class="radio radio-sm"
          name={`${id}-colours`}
          checked={look.colourSet === set.id}
          onchange={() => chooseColourSet(set.id)}
        />
        <span>{set.label}</span>
      </label>
    {/each}
  </fieldset>

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
