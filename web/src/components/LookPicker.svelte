<script lang="ts">
  // The choice of the look of each person: the layout (OG Organizr or the modern one), and
  // for it the theme and, on the modern layout, its colours. It is plain markup, so the menu of the
  // user of the OG Organizr layout, the Account page and the popover of the modern top bar
  // can all show it. Each change is saved at once and the page then loads the look again.
  // The radios stay enabled while a save runs, so that the keyboard focus stays in them.
  import { tick } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { MODERN_THEMES, modernThemeId } from '../lib/look';
  import { session } from '../lib/session.svelte';
  import { NATIVE_THEMES, swatchColors, themeId, themeLabel } from '../lib/theme';
  import ThemeSwatch from './ThemeSwatch.svelte';

  const id = $props.id();

  let error = $state<string | null>(null);
  let busy = $state(false);
  // Changes when a save fails, so that the controls show the saved look again.
  let generation = $state(0);
  let root: HTMLDivElement | undefined = $state();

  type Choice = { layout: string | null; theme?: string | null; colorSet?: string | null };
  // The last choice that is not saved yet. A choice made while a save runs waits here and
  // goes next, so that the arrow keys can run through the themes: only the last one counts.
  let wanted = $state<Choice | null>(null);

  const look = $derived(session.look);
  // The look that the controls show: the saved one, or the choice of a theme or colours that
  // is being saved. A new layout shows its themes once it is saved.
  const shown = $derived(
    wanted && wanted.layout && wanted.theme && wanted.colorSet
      ? { layout: wanted.layout, theme: wanted.theme, colorSet: wanted.colorSet }
      : look,
  );
  const shownLayout = $derived(wanted?.layout ?? look.layout);
  const modern = $derived(shown.layout === 'modern');
  const COLOR_SETS = [
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
          swatch: modernThemeId(theme.id, colorSetFor(theme.id)),
        }))
      : NATIVE_THEMES.map((name) => ({ id: name, label: themeLabel(name), swatch: themeId(name) })),
  );
  // On OG, the sample of the install's own theme shows the Admin's colours of "Colors &
  // Themes" over its own, as the page does. Other samples keep their stock colours.
  const installTheme = $derived(session.launch?.theme);
  const adminColors = $derived(swatchColors(session.launch?.appearance ?? {}));
  const colorsOf = (theme: string) => (!modern && theme === installTheme ? adminColors : undefined);
  const colorSets = $derived(
    modern
      ? (MODERN_THEMES.find((theme) => theme.id === shown.theme)?.colorSets ?? COLOR_SETS)
      : COLOR_SETS,
  );

  /** The name of the radio group that has the focus, if the focus is in this picker. */
  function focusedGroup(): string | null {
    const active = document.activeElement;
    return active instanceof HTMLInputElement && root?.contains(active) ? active.name : null;
  }

  async function send(body: Choice) {
    wanted = body;
    error = null;
    if (busy) {
      // The save that runs sends this choice when it ends.
      return;
    }
    busy = true;
    const group = focusedGroup();
    try {
      while (wanted) {
        const next: Choice = wanted;
        try {
          await api('PUT', 'user/look', next);
          await session.load();
        } catch (failure) {
          error = failureText(failure);
          // A choice that waited is dropped too: the alert says why, and the controls show
          // the saved look again.
          wanted = null;
          generation += 1;
          break;
        }
        if (wanted === next) {
          wanted = null;
        }
      }
    } finally {
      busy = false;
    }
    // The radio of the saved choice keeps the focus, also when the controls were drawn again.
    if (group) {
      await tick();
      if (!root?.contains(document.activeElement)) {
        root
          ?.querySelector<HTMLInputElement>(`input[name="${group}"]:checked`)
          ?.focus({ preventScroll: true });
      }
    }
  }

  /** The layout changes; its theme and colours stay as the person last chose them there. */
  const chooseLayout = (layout: 'og' | 'modern') => {
    if (layout !== shownLayout) {
      return send({ layout });
    }
  };

  /** The colour set of a theme after a choice of it: the current one if the theme has it. */
  function colorSetFor(theme: string): string {
    // A colour set that the new theme does not have gives way to the first one it has.
    const sets = modern
      ? (MODERN_THEMES.find((entry) => entry.id === theme)?.colorSets ?? COLOR_SETS)
      : COLOR_SETS;
    return sets.some((set) => set.id === shown.colorSet)
      ? shown.colorSet
      : (sets[0]?.id ?? shown.colorSet);
  }

  /**
   * Whether a change of the layout waits or runs. The themes and colours shown are still
   * those of the old layout until it lands, so a choice among them is not taken: it would
   * be sent with the old layout and undo the change.
   */
  const switching = $derived(shownLayout !== shown.layout);

  /** Puts the check of a radio group back on the radio of the shown look. */
  function keepShown(input: HTMLInputElement, value: string) {
    input.checked = false;
    const radios = root?.querySelectorAll<HTMLInputElement>(`input[name="${input.name}"]`);
    for (const radio of radios ?? []) {
      radio.checked = radio.value === value;
    }
  }

  function chooseTheme(theme: string, input: HTMLInputElement) {
    if (switching) {
      keepShown(input, shown.theme);
      return;
    }
    return send({ layout: shown.layout, theme, colorSet: colorSetFor(theme) });
  }

  function chooseColorSet(colorSet: string, input: HTMLInputElement) {
    if (switching) {
      keepShown(input, shown.colorSet);
      return;
    }
    return send({ layout: shown.layout, theme: shown.theme, colorSet });
  }
</script>

<div class="flex flex-col gap-3" bind:this={root}>
  {#key generation}
    <fieldset class="flex flex-col gap-1" aria-busy={busy}>
      <legend class="mb-1 text-sm font-semibold">Layout</legend>
      {#each [{ id: 'og', label: 'OG Organizr' }, { id: 'modern', label: 'Modern' }] as const as layout (layout.id)}
        <label class="flex items-center gap-2">
          <input
            type="radio"
            class="radio radio-sm"
            name={`${id}-layout`}
            checked={shownLayout === layout.id}
            onchange={() => chooseLayout(layout.id)}
          />
          <span>{layout.label}</span>
        </label>
      {/each}
    </fieldset>

    <fieldset class="flex flex-col gap-1" aria-busy={busy}>
      <legend class="mb-1 text-sm font-semibold">Theme</legend>
      <!-- Two columns when the picker has the room for both at their longest words, else
           one; a long list scrolls in the popover. The room is that of the picker, not of
           the window, so a large text size in a narrow popover gets one column. -->
      <div class="@container">
        <div
          class="grid max-h-[50vh] grid-cols-1 gap-x-3 gap-y-1 overflow-y-auto p-0.5 @min-[19.5rem]:grid-cols-[repeat(2,minmax(min-content,1fr))]"
        >
          {#each themes as theme (theme.id)}
            <!-- A column is at least as wide as its longest word, so a name never breaks
                 inside a word; a longer name wraps between its words. -->
            <label class="flex items-center gap-2">
              <input
                type="radio"
                class="radio radio-sm"
                name={`${id}-theme`}
                value={theme.id}
                checked={shown.theme === theme.id}
                onchange={(event) => chooseTheme(theme.id, event.currentTarget)}
              />
              <ThemeSwatch theme={theme.swatch} colors={colorsOf(theme.id)} />
              <span class="min-w-0 wrap-break-word">{theme.label}</span>
            </label>
          {/each}
        </div>
      </div>
    </fieldset>

    {#if modern}
      <fieldset class="flex flex-col gap-1" aria-busy={busy}>
        <legend class="mb-1 text-sm font-semibold">Colors</legend>
        {#each colorSets as set (set.id)}
          <label class="flex items-center gap-2">
            <input
              type="radio"
              class="radio radio-sm"
              name={`${id}-colors`}
              value={set.id}
              checked={shown.colorSet === set.id}
              disabled={colorSets.length < 2}
              onchange={(event) => chooseColorSet(set.id, event.currentTarget)}
            />
            <ThemeSwatch theme={modernThemeId(shown.theme, set.id)} />
            <span class="min-w-0 wrap-break-word">{set.label}</span>
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
      onclick={() => send({ layout: null, theme: null, colorSet: null })}
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
